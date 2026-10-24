//! Copy the managed files of a harness source into a project, and detect drift.
//!
//! A source holds `files/` (managed: overwritten on every sync while untouched locally) and `seed/`
//! (project-owned: copied once, never again). `.claude/harness.lock` records the hash of each managed file as
//! written, so a later sync tells a local edit from a source change.

use std::collections::{BTreeMap, BTreeSet};
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
        })
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub entries: Vec<(String, Outcome)>,
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

/// The lock of the project, `None` when it has none yet.
fn read_lock(root: &Path) -> Result<Option<BTreeMap<String, String>>> {
    let path = root.join(LOCK);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let mut lock = BTreeMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let Some((h, rel)) = line.split_once("  ") else {
            continue;
        };
        if !is_inside(rel) {
            bail!("{}: path outside the project: {rel}", path.display());
        }
        lock.insert(rel.to_string(), h.to_string());
    }
    Ok(Some(lock))
}

fn write_lock(root: &Path, lock: &BTreeMap<String, String>) -> Result<()> {
    let mut text =
        String::from("# Written by `harness sync`: hashes of the managed files. Do not edit.\n");
    for (path, h) in lock {
        text.push_str(&format!("{h}  {path}\n"));
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

/// Clone (first time) or fetch the git source in `cache`, and return its local directory.
fn fetch(cache: &Path, url: &str) -> Result<PathBuf> {
    let dir = cache.join("harness").join(&hash(url.as_bytes())[..12]);
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
    let target = candidates
        .iter()
        .find(|c| resolves(dir, c))
        .with_context(|| format!("{url}: unknown revision {}", rev.unwrap_or("origin/HEAD")))?;
    git_output(dir, &["rev-parse", &format!("{target}^{{commit}}")])
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
    sync_from(root, &source_dir(None, root, None)?, options)
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
    let lock = read_lock(root)?.unwrap_or_default();
    let mut new_lock = BTreeMap::new();
    let mut report = Report::default();

    let files = list(&managed)?;
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

    if !options.dry_run {
        write_lock(root, &new_lock)?;
    }
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
/// the resolved commit to `rev` in `harness.toml`. A conflict, an unknown revision or an unreadable source leave the project as it was;
/// an I/O error during the write itself is not rolled back, as for `sync`.
///
/// # Errors
/// When the source is a local directory, the revision is unknown, or the configuration or sync fails.
pub fn update(root: &Path, rev: Option<&str>, options: &SyncOptions) -> Result<UpdateReport> {
    update_in(None, root, rev, options)
}

fn update_in(
    cache: Option<&Path>,
    root: &Path,
    rev: Option<&str>,
    options: &SyncOptions,
) -> Result<UpdateReport> {
    let (source, previous) = source_and_rev(root)?;
    if !is_git_url(&source) {
        bail!("{CONFIG}: `source` is a local directory, which has no revision to update");
    }
    let dir = fetch(&cache_or_default(cache)?, &source)?;
    let commit = resolve_in(&dir, &source, rev)?;
    git(Some(&dir), &["checkout", "--quiet", "--detach", &commit])?;
    let path = root.join(CONFIG);
    let text = set_rev(
        &fs::read_to_string(&path)?,
        &commit,
        rev.unwrap_or("default branch"),
    )?;
    // Nothing is written before the dry run shows a clean sync.
    let preview = sync_from(
        root,
        &dir,
        &SyncOptions {
            dry_run: true,
            ..*options
        },
    )?;
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

/// Managed files modified or deleted since the last sync, one line each.
///
/// # Errors
/// When the project was never synced (no lock file) or the lock cannot be read.
pub fn check(root: &Path) -> Result<Vec<String>> {
    let Some(lock) = read_lock(root)? else {
        bail!("no {LOCK}: run `harness sync` first");
    };
    Ok(lock
        .iter()
        .filter_map(|(rel, locked)| match fingerprint_of(&root.join(rel)) {
            None => Some(format!("missing: {rel}")),
            Some(print) if print != *locked => Some(format!("modified: {rel}")),
            Some(_) => None,
        })
        .collect())
}

#[cfg(test)]
mod tests;
