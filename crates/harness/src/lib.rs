//! Copy the managed files of a harness source into a project, and detect drift.
//!
//! A source holds `files/` (managed: overwritten on every sync while untouched locally) and `seed/`
//! (project-owned: copied once, never again), and may hold `entries/.claude/settings.json`, the entries of the
//! project's settings it manages (`entries.rs`). `.claude/harness.lock` records the hash of each managed file as
//! written and the managed entries, so a later sync tells a local edit from a source change.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

/// Project configuration, relative to the project root.
pub const CONFIG: &str = ".claude/harness.toml";
/// Hashes of the managed files as last written, relative to the project root.
pub const LOCK: &str = ".claude/harness.lock";

mod entries;
mod gate;
mod launcher;
pub mod run;
mod settings;
pub mod tools;

#[derive(Clone, Copy, Debug, Default)]
pub struct SyncOptions {
    pub force: bool,
    pub dry_run: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Added,
    Updated,
    Removed,
    Seeded,
    Unchanged,
    /// Managed file modified locally: left as it is.
    Conflict,
    /// File dropped from the source but modified locally: left as it is, no longer managed.
    Kept,
    /// A hook of the project's own moved aside so that the managed one can chain it.
    Moved,
    /// Not done, for the reason given.
    Skipped(&'static str),
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Outcome::Added => "added",
            Outcome::Updated => "updated",
            Outcome::Removed => "removed",
            Outcome::Seeded => "seeded",
            Outcome::Unchanged => "unchanged",
            Outcome::Conflict => "conflict (modified locally, fix it in the source or use --force)",
            Outcome::Kept => "kept (dropped from the source but modified locally)",
            Outcome::Moved => "moved (your hook now runs after the review gate)",
            Outcome::Skipped(why) => return write!(f, "skipped ({why})"),
        })
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub entries: Vec<(String, Outcome)>,
    /// What the project's settings lack or run in vain once synced (SYN-27): printed, never a failure.
    pub warnings: Vec<String>,
}

impl Report {
    #[must_use]
    pub fn conflicts(&self) -> usize {
        self.entries
            .iter()
            .filter(|(_, o)| *o == Outcome::Conflict)
            .count()
    }

    #[must_use]
    pub fn outcome(&self, path: &str) -> Option<Outcome> {
        self.entries
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, o)| *o)
    }
}

/// Hash of `bytes`, followed by `+x` for an executable file: a mode change is a change.
fn fingerprint(bytes: &[u8], executable: bool) -> String {
    let mut print = hash(bytes);
    if executable {
        print.push_str("+x");
    }
    print
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    false
}

/// Fingerprint of the file at `path`, `None` when it cannot be read.
fn fingerprint_of(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(fingerprint(&bytes, is_executable(path)))
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            s.push_str(&format!("{b:02x}"));
            s
        })
}

/// Relative paths (with `/`) of every file under `dir`, sorted. A missing `dir` is empty.
fn list(dir: &Path) -> Result<Vec<String>> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
            let path = entry?.path();
            if path.is_dir() {
                walk(base, &path, out)?;
            } else if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    if dir.is_dir() {
        walk(dir, dir, &mut out)?;
    }
    out.sort();
    Ok(out)
}

/// A path of the lock must stay inside the project: relative, no `..`.
fn is_inside(rel: &str) -> bool {
    Path::new(rel)
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
}

/// What the lock records: the fingerprint of each managed file, and the managed entries of the settings.
#[derive(Debug, Default)]
struct Lock {
    files: BTreeMap<String, String>,
    entries: Vec<entries::Entry>,
}

/// The mark of a lock line that holds a managed entry of the settings instead of a file.
const ENTRY: &str = "entry";

/// The lock of the project, `None` when it has none yet.
fn read_lock(root: &Path) -> Result<Option<Lock>> {
    let path = root.join(LOCK);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let mut lock = Lock::default();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let Some((h, rel)) = line.split_once("  ") else {
            continue;
        };
        if h == ENTRY {
            let entry = entries::Entry::from_lock(rel)
                .with_context(|| format!("reading {}", path.display()))?;
            lock.entries.push(entry);
            continue;
        }
        if !is_inside(rel) {
            bail!("{}: path outside the project: {rel}", path.display());
        }
        lock.files.insert(rel.to_string(), h.to_string());
    }
    Ok(Some(lock))
}

fn write_lock(root: &Path, lock: &Lock) -> Result<()> {
    let mut text = String::from(
        "# Written by `harness sync`: hashes of the managed files, then the managed settings entries. Do not edit.\n",
    );
    for (path, h) in &lock.files {
        text.push_str(&format!("{h}  {path}\n"));
    }
    for entry in &lock.entries {
        text.push_str(&format!("{ENTRY}  {}\n", entry.to_lock()));
    }
    write(&root.join(LOCK), text.as_bytes())
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}

/// `write`, then make the file executable or not as in the source.
fn write_as(path: &Path, bytes: &[u8], executable: bool) -> Result<()> {
    write(path, bytes)?;
    set_executable(path, executable)
}

#[cfg(unix)]
fn set_executable(path: &Path, executable: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    let mode = permissions.mode();
    // Only the execute bits change, and only where the read bits are: the usual `chmod +x` / `-x`.
    let mode = if executable {
        mode | ((mode & 0o444) >> 2)
    } else {
        mode & !0o111
    };
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions).with_context(|| format!("chmod {}", path.display()))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path, _executable: bool) -> Result<()> {
    Ok(())
}

fn read_config(root: &Path) -> Result<toml::Table> {
    let path = root.join(CONFIG);
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "no {CONFIG} in {}: run `harness init <source>`",
            root.display()
        )
    })?;
    text.parse()
        .with_context(|| format!("parsing {}", path.display()))
}

fn is_git_url(source: &str) -> bool {
    source.contains("://") || source.starts_with("git@") || source.ends_with(".git")
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    let status = cmd.args(args).status().context("running git")?;
    if !status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(())
}

fn resolves(dir: &Path, rev: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ])
        .output()
        .is_ok_and(|o| o.status.success())
}

fn git_output(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .context("running git")?;
    if !out.status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn cache_dir() -> Result<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .context("no cache directory (set HOME or XDG_CACHE_HOME)")
}

fn cache_or_default(cache: Option<&Path>) -> Result<PathBuf> {
    cache.map_or_else(cache_dir, |c| Ok(c.to_path_buf()))
}

/// Where the clone of the git source `url` lives in `cache`.
fn clone_dir(cache: &Path, url: &str) -> PathBuf {
    cache.join("harness").join(&hash(url.as_bytes())[..12])
}

/// Clone (first time) or fetch the git source in `cache`, and return its local directory.
fn fetch(cache: &Path, url: &str) -> Result<PathBuf> {
    let dir = clone_dir(cache, url);
    if dir.join(".git").exists() {
        git(Some(&dir), &["fetch", "--quiet", "--tags", "origin"])?;
        // `fetch` leaves `origin/HEAD` alone: follow a default branch the remote renamed (best effort).
        let _ = git_output(&dir, &["remote", "set-head", "origin", "--auto"]);
    } else {
        fs::create_dir_all(&dir)?;
        git(None, &["clone", "--quiet", url, &dir.to_string_lossy()])?;
    }
    Ok(dir)
}

/// Full commit hash of `rev` (default: the remote's default branch) in a fetched source.
fn resolve_in(dir: &Path, url: &str, rev: Option<&str>) -> Result<String> {
    if rev.is_some_and(|r| r.starts_with('-')) {
        bail!("{url}: invalid revision {}", rev.unwrap_or_default());
    }
    // A branch name must resolve to the remote branch the fetch just moved, not to a stale local one.
    let candidates = match rev {
        Some(rev) => vec![format!("origin/{rev}"), rev.to_string()],
        None => vec!["origin/HEAD".to_string()],
    };
    if let Some(target) = candidates.iter().find(|c| resolves(dir, c)) {
        return git_output(dir, &["rev-parse", &format!("{target}^{{commit}}")]);
    }
    let rev = rev.unwrap_or("origin/HEAD");
    if !is_full_hash(rev) {
        bail!("{url}: unknown revision {rev}");
    }
    // No branch or tag points to the pinned commit any more: ask the source for it by hash (hosts differ
    // on whether they serve it; a refusal is reported below).
    let _ = git(Some(dir), &["fetch", "--quiet", "origin", rev]);
    if !resolves(dir, rev) {
        bail!("{url}: the pinned commit {rev} is gone from the source (or the source is unreachable: see git's message above); run `harness update` to pin a current revision");
    }
    git_output(dir, &["rev-parse", &format!("{rev}^{{commit}}")])
}

/// A 40 (SHA-1) or 64 (SHA-256) hexadecimal digit commit name, as `update` writes it to `rev`.
fn is_full_hash(rev: &str) -> bool {
    matches!(rev.len(), 40 | 64) && rev.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Local checkout of a git source in the user cache, at `rev` (default: the remote's default branch).
fn checkout(cache: Option<&Path>, url: &str, rev: Option<&str>) -> Result<PathBuf> {
    let dir = fetch(&cache_or_default(cache)?, url)?;
    let commit = resolve_in(&dir, url, rev)?;
    git(Some(&dir), &["checkout", "--quiet", "--detach", &commit])?;
    Ok(dir)
}

/// `harness.toml` text with `rev` set in the `[harness]` table, every other line left as it is.
///
/// The line becomes `rev = "<commit>"  # <name>`; a missing `rev` is inserted right after `source`.
fn set_rev(text: &str, commit: &str, name: &str) -> Result<String> {
    if name.contains(char::is_control) {
        bail!("invalid revision name {name:?}");
    }
    let line = if name == commit {
        format!("rev = \"{commit}\"")
    } else {
        format!("rev = \"{commit}\"  # {name}")
    };
    let key_is = |l: &str, key: &str| {
        l.trim_start()
            .strip_prefix(key)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    };
    let mut in_harness = false;
    let mut source_at = None;
    let mut rev_at = None;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    for (i, l) in lines.iter().enumerate() {
        let trimmed = l.trim();
        if is_table_header(trimmed) {
            in_harness = trimmed.starts_with("[harness]");
        } else if in_harness && key_is(l, "rev") {
            rev_at = Some(i);
        } else if in_harness && key_is(l, "source") {
            source_at = Some(i);
        }
    }
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out: Vec<String> = lines.iter().map(|l| (*l).to_string()).collect();
    if let Some(i) = rev_at {
        let ending = &lines[i][lines[i].trim_end_matches(['\r', '\n']).len()..];
        out[i] = format!("{line}{ending}");
    } else if let Some(i) = source_at {
        if !out[i].ends_with('\n') {
            out[i].push_str(eol);
        }
        out.insert(i + 1, format!("{line}{eol}"));
    } else {
        bail!("{CONFIG}: missing `source` in the [harness] table");
    }
    let out = out.concat();
    // Quoted or dotted keys are valid TOML this line-based edit does not see: refuse rather than corrupt.
    let written = out
        .parse::<toml::Table>()
        .ok()
        .and_then(|t| t.get("harness")?.get("rev")?.as_str().map(|r| r == commit));
    if written != Some(true) {
        bail!("{CONFIG}: cannot rewrite `rev` automatically, set `rev = \"{commit}\"` in [harness] by hand");
    }
    Ok(out)
}

/// A `[table]` line, as opposed to a continuation line of a multi-line array such as `["a"],`.
fn is_table_header(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('[') else {
        return false;
    };
    let Some((name, after)) = rest.split_once(']') else {
        return false;
    };
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ' '))
        && (after.trim().is_empty() || after.trim_start().starts_with('#'))
}

/// The `source` and the pinned `rev` of the `[harness]` table.
fn source_and_rev(root: &Path) -> Result<(String, Option<String>)> {
    let config = read_config(root)?;
    let table = config.get("harness").and_then(toml::Value::as_table);
    let text = |key: &str| {
        table
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    };
    let source = text("source")
        .with_context(|| format!("{CONFIG}: missing `source` in the [harness] table"))?;
    Ok((source, text("rev")))
}

fn source_dir(cache: Option<&Path>, root: &Path, rev_override: Option<&str>) -> Result<PathBuf> {
    let (source, rev) = source_and_rev(root)?;
    if is_git_url(&source) {
        checkout(cache, &source, rev_override.or(rev.as_deref()))
    } else {
        Ok(root.join(source))
    }
}

/// Create `.claude/harness.toml` from the source and the project-owned defaults of the source.
///
/// # Errors
/// When the project already has a `harness.toml` or the source cannot be read.
pub fn init(root: &Path, source: &str, rev: Option<&str>) -> Result<()> {
    let path = root.join(CONFIG);
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    let mut harness = toml::Table::new();
    harness.insert("source".into(), source.into());
    if let Some(rev) = rev {
        harness.insert("rev".into(), rev.into());
    }
    let mut config = toml::Table::new();
    config.insert("harness".into(), harness.into());
    let mut text = config.to_string();
    let dir = if is_git_url(source) {
        checkout(None, source, rev)?
    } else {
        root.join(source)
    };
    if let Ok(defaults) = fs::read_to_string(dir.join("seed").join(CONFIG)) {
        text.push('\n');
        text.push_str(&defaults);
    }
    write(&path, text.as_bytes())
}

/// Bring the project in step with its source.
///
/// # Errors
/// When the configuration or the source cannot be read, or a file cannot be written.
pub fn sync(root: &Path, options: &SyncOptions) -> Result<Report> {
    let dir = source_dir(None, root, None)?;
    let mut report = sync_from(root, &dir, options)?;
    let (source, _) = source_and_rev(root)?;
    if is_git_url(&source) && !options.dry_run {
        let commit = git_output(&dir, &["rev-parse", "HEAD"])?;
        report.warnings.extend(prefetch(
            &cache_dir()?,
            &source,
            &commit,
            &run::Programs::default(),
        ));
    }
    Ok(report)
}

/// Fetch the tools binary of `commit` into the cache, as `harness run` would: a warning when it fails (BIN-13).
fn prefetch(cache: &Path, source: &str, commit: &str, programs: &run::Programs) -> Option<String> {
    run::binary_of(cache, source, commit, programs)
        .err()
        .map(|e| format!("warning: {e:#} (`harness run` will try again)"))
}

fn sync_from(root: &Path, source: &Path, options: &SyncOptions) -> Result<Report> {
    let managed = source.join("files");
    let seeds = source.join("seed");
    if !managed.is_dir() {
        bail!(
            "{} has no files/ directory: is `source` right in {CONFIG}?",
            source.display()
        );
    }
    let Lock {
        files: lock,
        entries: recorded,
    } = read_lock(root)?.unwrap_or_default();
    let wanted = entries::of_source(source)?;
    // decided first: a hook of the user that cannot be chained stops the sync before it writes anything
    let files = list(&managed)?;
    let gate = gate::plan(root, gate::manages(&wanted, files.iter()))?;
    // installed before the managed files are written, so a failure leaves the project as it was
    if let (Some(gate), false) = (&gate, options.dry_run) {
        gate.install()?;
    }
    let mut new_lock = BTreeMap::new();
    let mut report = Report::default();

    let file_set: BTreeSet<&String> = files.iter().collect();
    for rel in &files {
        let bytes = fs::read(managed.join(rel))?;
        let executable = is_executable(&managed.join(rel));
        let want = fingerprint(&bytes, executable);
        let have = fingerprint_of(&root.join(rel));
        let locked = lock.get(rel);
        let outcome = match &have {
            None => Outcome::Added,
            Some(h) if *h == want => Outcome::Unchanged,
            Some(h) if Some(h) == locked || options.force => Outcome::Updated,
            Some(_) => Outcome::Conflict,
        };
        match outcome {
            Outcome::Added | Outcome::Updated => {
                if !options.dry_run {
                    write_as(&root.join(rel), &bytes, executable)?;
                }
                new_lock.insert(rel.clone(), want);
            }
            Outcome::Unchanged => {
                new_lock.insert(rel.clone(), want);
            }
            _ => {
                if let Some(old) = locked {
                    new_lock.insert(rel.clone(), old.clone());
                }
            }
        }
        report.entries.push((rel.clone(), outcome));
    }

    for (rel, locked) in lock.iter().filter(|(rel, _)| !file_set.contains(rel)) {
        let path = root.join(rel);
        match fingerprint_of(&path) {
            None => {}
            Some(h) if h == *locked => {
                if !options.dry_run {
                    fs::remove_file(&path)?;
                }
                report.entries.push((rel.clone(), Outcome::Removed));
            }
            Some(_) => report.entries.push((rel.clone(), Outcome::Kept)),
        }
    }

    for rel in list(&seeds)? {
        let dst = root.join(&rel);
        // `symlink_metadata`: a dangling symlink is something the project has. A managed path is not a seed.
        if file_set.contains(&rel) || fs::symlink_metadata(&dst).is_ok() {
            continue;
        }
        if !options.dry_run {
            write_as(
                &dst,
                &fs::read(seeds.join(&rel))?,
                is_executable(&seeds.join(&rel)),
            )?;
        }
        report.entries.push((rel, Outcome::Seeded));
    }

    // after the seeds: a first sync merges into the settings it was just seeded with, written or not
    let seeded = report.outcome(entries::SETTINGS) == Some(Outcome::Seeded);
    let base = if seeded { &seeds } else { root };
    let text = fs::read_to_string(base.join(entries::SETTINGS)).ok();
    // settings seeded again hold nothing of the project's: no recorded entry is in conflict there
    let recorded = if seeded { Vec::new() } else { recorded };
    let merge = entries::merge(text.as_deref(), &wanted, &recorded, options.force);
    if let (Some(text), false) = (&merge.write, options.dry_run) {
        write(&root.join(entries::SETTINGS), text.as_bytes())?;
    }
    report.entries.extend(merge.outcomes);

    if let Some(gate) = gate {
        report.entries.extend(gate.outcomes());
    }

    let new_lock = Lock {
        files: new_lock,
        entries: merge.lock,
    };
    if !options.dry_run {
        write_lock(root, &new_lock)?;
    }
    // judged on the project as the sync leaves it, or would under `--dry-run`
    let done = |path: &str, outcome| report.outcome(path) == Some(outcome);
    let present = |path: &str| {
        done(path, Outcome::Added) || (root.join(path).is_file() && !done(path, Outcome::Removed))
    };
    let gated = gate::manages(&new_lock.entries, new_lock.files.keys());
    report.warnings = match &merge.settings {
        Some(settings) => settings::warnings_of(settings, gated, &present),
        None => settings::warnings(base, gated, &present),
    };
    report.warnings.extend(merge.warning);
    Ok(report)
}

/// What `update` did: the revision before and after, and the `sync` that followed.
#[derive(Debug)]
pub struct UpdateReport {
    /// The `rev` the project had, if any.
    pub previous: Option<String>,
    /// The commit the project was moved to, or would be.
    pub rev: String,
    /// False when nothing was written: `--dry-run`, or a conflict.
    pub applied: bool,
    pub sync: Report,
}

/// Move the project to `rev` of its git source (default: the head of the default branch): sync to it, then write
/// the resolved commit to `rev` in `harness.toml`. A conflict, an unknown revision, an unreadable source or a hook the `harness` on the path refuses (BIN-16) leave the project as it was;
/// an I/O error during the write itself is not rolled back, as for `sync`.
///
/// # Errors
/// When the source is a local directory, the revision is unknown, or the configuration or sync fails.
pub fn update(root: &Path, rev: Option<&str>, options: &SyncOptions) -> Result<UpdateReport> {
    let mut done = update_in(None, root, rev, options, OsStr::new("harness"))?;
    if done.applied {
        let (source, _) = source_and_rev(root)?;
        done.sync.warnings.extend(prefetch(
            &cache_dir()?,
            &source,
            &done.rev,
            &run::Programs::default(),
        ));
    }
    Ok(done)
}

fn update_in(
    cache: Option<&Path>,
    root: &Path,
    rev: Option<&str>,
    options: &SyncOptions,
    harness: &OsStr,
) -> Result<UpdateReport> {
    let (source, previous) = source_and_rev(root)?;
    if !is_git_url(&source) {
        bail!("{CONFIG}: `source` is a local directory, which has no revision to update");
    }
    let dir = fetch(&cache_or_default(cache)?, &source)?;
    let commit = resolve_in(&dir, &source, rev)?;
    git(Some(&dir), &["checkout", "--quiet", "--detach", &commit])?;
    // The hooks of the target must not lock the agent out: the installed launcher has to know their subcommands. A
    // fragment that cannot be read fails the sync below, before anything is written.
    let refused: Vec<String> = fs::read_to_string(dir.join(entries::FRAGMENT))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .map(|fragment| launcher::refused(&fragment, harness))
        .unwrap_or_default()
        .iter()
        .map(|command| {
            format!(
                "the installed harness cannot run the hook `{command}` of {commit}: install the harness of {commit} first, `cargo install --git {} --rev {commit} --locked`, then run `harness update` again",
                run::cargo_url(&source)
            )
        })
        .collect();
    if !refused.is_empty() && !options.dry_run {
        bail!("{}", refused.join("\n"));
    }
    let path = root.join(CONFIG);
    let text = set_rev(
        &fs::read_to_string(&path)?,
        &commit,
        rev.unwrap_or("default branch"),
    )?;
    // Nothing is written before the dry run shows a clean sync.
    let mut preview = sync_from(
        root,
        &dir,
        &SyncOptions {
            dry_run: true,
            ..*options
        },
    )?;
    preview
        .warnings
        .extend(refused.iter().map(|line| format!("warning: {line}")));
    let applied = !options.dry_run && preview.conflicts() == 0;
    let sync = if applied {
        let sync = sync_from(root, &dir, options)?;
        write(&path, text.as_bytes())?;
        sync
    } else {
        preview
    };
    Ok(UpdateReport {
        previous,
        rev: commit,
        applied,
        sync,
    })
}

/// What `check` found: `problems` make it fail, `warnings` do not.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Checked {
    /// Managed files modified or deleted since the last sync, and the native review gate when the project should have
    /// it and does not, and the hook commands the `harness` on the path refuses (BIN-15).
    pub problems: Vec<String>,
    /// What the project's settings lack or run in vain (SYN-24 to SYN-26).
    pub warnings: Vec<String>,
}

/// Managed files modified or deleted since the last sync, one line each, and the native review gate when the project
/// should have it and does not. Under CI (the `CI` environment variable is set) the gate is not looked at: a fresh
/// clone has no git hooks, and nobody commits there. It also warns of what the project's settings lack: the deny rules of
/// the review waiver (when the review hook is among the managed files), the hooks they run, the attribution turned off.
/// A hook command whose subcommand the `harness` on the path refuses is a problem too (BIN-15).
///
/// # Errors
/// When the project was never synced (no lock file) or the lock cannot be read.
pub fn check(root: &Path) -> Result<Checked> {
    check_with(root, in_ci(), OsStr::new("harness"))
}

/// Whether `CI` says so: set, and not empty, `0` or `false`.
fn in_ci() -> bool {
    std::env::var("CI")
        .is_ok_and(|v| !matches!(v.trim().to_lowercase().as_str(), "" | "0" | "false"))
}

/// The warnings on the retention of the transcripts (SYN-28), printed by `check`, `sync` and `update`; none under CI.
/// Kept out of `check` and `sync`, which read the project only: these read the user settings too.
#[must_use]
pub fn retention_warnings(root: &Path) -> Vec<String> {
    let user = user_settings(
        std::env::var_os("CLAUDE_CONFIG_DIR"),
        std::env::var_os("HOME"),
    );
    retention_with(root, in_ci(), user.as_deref())
}

fn retention_with(root: &Path, ci: bool, user: Option<&Path>) -> Vec<String> {
    if ci {
        return Vec::new();
    }
    settings::retention(root, user)
}

/// The user settings of Claude Code: `settings.json` in `config_dir` (`CLAUDE_CONFIG_DIR`), or else in `~/.claude`;
/// `None` when neither is set.
fn user_settings(config_dir: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let set = |var: Option<OsString>| var.filter(|v| !v.is_empty()).map(PathBuf::from);
    set(config_dir)
        .or_else(|| set(home).map(|h| h.join(".claude")))
        .map(|dir| dir.join("settings.json"))
}

fn check_with(root: &Path, ci: bool, harness: &OsStr) -> Result<Checked> {
    let Some(lock) = read_lock(root)? else {
        bail!("no {LOCK}: run `harness sync` first");
    };
    let gated = gate::manages(&lock.entries, lock.files.keys());
    let settings = fs::read_to_string(root.join(entries::SETTINGS)).ok();
    let hooks = settings
        .as_deref()
        .and_then(|text| serde_json::from_str(text).ok())
        .map(|settings| launcher::refused(&settings, harness))
        .unwrap_or_default();
    let problems = lock
        .files
        .iter()
        .filter_map(|(rel, locked)| match fingerprint_of(&root.join(rel)) {
            None => Some(format!("missing: {rel}")),
            Some(print) if print != *locked => Some(format!("modified: {rel}")),
            Some(_) => None,
        })
        .chain(entries::missing(settings.as_deref(), &lock.entries))
        .chain(gate::problem(root, !ci && gated))
        .chain(hooks.iter().map(|command| {
            format!(
                "refused: the installed harness cannot run the hook `{command}` of {}: install the harness of this revision first",
                entries::SETTINGS
            )
        }))
        .collect();
    Ok(Checked {
        problems,
        warnings: settings::warnings(root, gated, &|path| root.join(path).is_file()),
    })
}

#[cfg(test)]
mod tests;
