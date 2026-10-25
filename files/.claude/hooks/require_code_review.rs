#!/usr/bin/env rust-script
//! Refuses a `git commit` until the code review has run on the current diff. Two modes, one per event:
//! `record` (PostToolUse on `Skill`) stores a marker when the `code-review` skill ends; `check` (PreToolUse
//! on `Bash`) compares the diff at `git commit` with the marker and exits 2 when the review is missing.
//!
//! The marker, `claude-code-review.json` in the git directory of the repository (so it is never a file of the
//! work tree, whatever the ignore rules), holds the `HEAD`, a hash of the diff against `HEAD` (tracked
//! changes and untracked files, the state a `git add -A && git commit` commits) and the files it covers.
//! A commit moves `HEAD`, which retires the marker. `code-review` runs forked, so its `PostToolUse` fires when
//! the review is over, not when the skill loads. Changes made afterwards (the fixes of the review) pass as long
//! as they stay in files the review saw. A commit with nothing to commit, an amend of a clean tree
//! included, one that only touches `issues/`, and a machine without `git` are not gated. The user waives the
//! review of the next commit by creating `claude-review-waived` in the git directory; the hook stamps it with
//! `HEAD` when it lets a commit through and removes it, without using it, at the first check after `HEAD` moved
//! (a command that does not commit, such as `--dry-run`, leaves it usable; after a commit the user creates it
//! again), and refuses any `Bash` command naming it (a deterrent, not a barrier: a `Write`
//! of the file or a name built in pieces is not caught).
//!
//! Declare: PostToolUse Skill (argument `record`), PreToolUse Bash (argument `check`)
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const REVIEW_SKILL: &str = "code-review";
/// A commit of files under this directory only (the issues) needs no review.
const ISSUES_DIR: &str = "issues/";
/// Created by the user, in the git directory, to waive the review of the next commit; the hook removes it.
const WAIVER: &str = "claude-review-waived";
/// Stamp of a waiver used in a repository without commit.
const NO_HEAD: &str = "no-head";

/// What the working tree holds relative to `HEAD`.
#[derive(Debug, PartialEq)]
struct Snapshot {
    head: String,
    hash: String,
    files: Vec<String>,
}

fn git(cwd: &Path, args: &[&str], stdin: &[u8]) -> Vec<u8> {
    let Ok(mut child) = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Vec::new();
    };
    let mut pipe = child.stdin.take().unwrap();
    let input = stdin.to_vec();
    let writer = std::thread::spawn(move || pipe.write_all(&input).ok());
    let out = child.wait_with_output().map(|o| o.stdout).unwrap_or_default();
    writer.join().ok();
    out
}

/// Names from `-z` output (NUL separated, never quoted by git).
fn names(bytes: &[u8]) -> Vec<String> {
    bytes.split(|b| *b == 0).filter(|n| !n.is_empty()).map(|n| String::from_utf8_lossy(n).into_owned()).collect()
}

fn snapshot(cwd: &Path) -> Snapshot {
    let head = String::from_utf8_lossy(&git(cwd, &["rev-parse", "-q", "--verify", "HEAD"], b"")).trim().to_string();
    // the empty tree stands for HEAD in a repository without commit
    let base = if head.is_empty() { "4b825dc642cb6eb9a060e54bf8d69288fbee4904" } else { "HEAD" };
    let mut content = git(cwd, &["diff", base, "--binary", "--no-color", "--no-ext-diff", "--no-textconv"], b"");
    let mut files = names(&git(cwd, &["diff", base, "--name-only", "--no-renames", "-z"], b""));
    for name in names(&git(cwd, &["ls-files", "--others", "--exclude-standard", "-z"], b"")) {
        let bytes = std::fs::read(cwd.join(&name)).unwrap_or_default();
        content.extend(format!("\0{name}\0{}\0", bytes.len()).into_bytes());
        content.extend(bytes);
        files.push(name);
    }
    files.sort();
    let hash = String::from_utf8_lossy(&git(cwd, &["hash-object", "--stdin"], &content)).trim().to_string();
    Snapshot { head, hash, files }
}

/// Global options of `git` that do not take the next word as value.
const SKIPPED_OPTIONS: [&str; 8] =
    ["--no-pager", "--paginate", "-p", "-P", "--no-optional-locks", "--bare", "--no-replace-objects", "--literal-pathspecs"];

/// Global options of `git` that take the next word as value; the same list as `GIT_VALUE_OPTIONS` of
/// `block_ai_attribution.rs` (a hook is a single file, keep the two in step). `--git-dir` and `--work-tree` are also
/// read apart, as `-C`: their value names the repository.
const GIT_VALUE_OPTIONS: [&str; 5] = ["--git-dir", "--work-tree", "--namespace", "--exec-path", "--super-prefix"];

/// Completes `first`, the start of a value, with the next words of `words` when it opens a quote it does not close.
fn quoted<'a>(first: &str, words: &mut impl Iterator<Item = &'a &'a str>) -> String {
    let mut value = first.to_string();
    if let Some(quote) = first.chars().next().filter(|c| matches!(c, '"' | '\'')) {
        if first.len() == 1 || !first.ends_with(quote) {
            for w in words.by_ref() {
                value = format!("{value} {w}");
                if w.ends_with(quote) {
                    break;
                }
            }
        }
    }
    value
}

/// Takes the value of an option from `words`: a quoted value may span several words.
fn take_value<'a>(words: &mut impl Iterator<Item = &'a &'a str>) -> Option<String> {
    let first = *words.next()?;
    Some(quoted(first, words))
}

/// The directories whose review a `git commit` of the line depends on, `None` when no command of the line commits;
/// `None` in the list is the shell directory. Normally one: after `-C dir`, `--work-tree dir` or `--git-dir dir` (the
/// work tree wins, then the parent of the innermost `.git` of the path, else the directory itself). With `--git-dir` and
/// no `--work-tree` there are two, since `git` commits the files of the directory it runs in into that git directory.
/// The `-C` options accumulate and a relative `--work-tree` or `--git-dir` is taken from the directory they lead to,
/// wherever they stand on the line, as `git` does. The subcommand is the first word after `git` once the global options
/// (`SKIPPED_OPTIONS`, `GIT_VALUE_OPTIONS`, `-C dir`, `-c key=value`) are skipped; any other option means it is not a
/// commit. Brackets, separators and quotes around `git` are ignored; `cd dir && git commit` is not followed.
fn commit_dirs(command: &str) -> Option<Vec<Option<String>>> {
    // every separator and bracket becomes a word `;` so that `a;git commit` and `(git commit)` split like `a ; git commit`
    let spaced = command.replace([';', '|', '&', '(', ')', '`'], " ; ");
    let words: Vec<&str> = spaced.split_whitespace().collect();
    words.iter().enumerate().find_map(|(i, w)| {
        if w.trim_start_matches(['$', '\'', '"']).rsplit('/').next() != Some("git") {
            return None;
        }
        let mut rest = words[i + 1..].iter().take_while(|w| **w != ";");
        let (mut dir, mut work_tree, mut git_dir) = (None::<PathBuf>, None::<PathBuf>, None::<PathBuf>);
        let path = |v: &str| PathBuf::from(v.trim_matches(['"', '\'']));
        loop {
            match rest.next() {
                Some(&"-C") => {
                    if let Some(v) = take_value(&mut rest) {
                        dir = Some(dir.map_or(path(&v), |d| d.join(path(&v))));
                    }
                }
                // `--git-dir` and `--work-tree` stay in `GIT_VALUE_OPTIONS` (same list as the other hook) but are read here first
                Some(&"--work-tree") => work_tree = take_value(&mut rest).map(|v| path(&v)),
                Some(&"--git-dir") => git_dir = take_value(&mut rest).map(|v| path(&v)),
                Some(w) if w.starts_with("--work-tree=") => work_tree = Some(path(&quoted(&w["--work-tree=".len()..], &mut rest))),
                Some(w) if w.starts_with("--git-dir=") => git_dir = Some(path(&quoted(&w["--git-dir=".len()..], &mut rest))),
                Some(w) if *w == "-c" || GIT_VALUE_OPTIONS.contains(w) => {
                    take_value(&mut rest);
                }
                Some(w) if SKIPPED_OPTIONS.contains(w) || GIT_VALUE_OPTIONS.iter().any(|o| w.strip_prefix(o).is_some_and(|v| v.starts_with('='))) => {}
                other => {
                    if other.map(|w| w.trim_end_matches(['\'', '"'])) != Some("commit") {
                        return None;
                    }
                    // the directory `-C` leads to is where the relative paths are taken from
                    let from_dir = |p: PathBuf| dir.as_ref().map_or(p.clone(), |d| d.join(&p));
                    let text = |p: PathBuf| Some(p.to_string_lossy().into_owned());
                    if let Some(w) = work_tree {
                        return Some(vec![text(from_dir(w))]);
                    }
                    let Some(g) = git_dir else { return Some(vec![dir.and_then(text)]) };
                    let g = from_dir(g);
                    // the repository holding the git directory: the parent of its innermost `.git`, else itself
                    let repo = g.ancestors().find(|a| a.file_name().is_some_and(|n| n == ".git")).and_then(Path::parent).map_or(g.clone(), Path::to_path_buf);
                    let repo = if repo.as_os_str().is_empty() { PathBuf::from(".") } else { repo };
                    return Some(vec![text(repo), dir.and_then(text)]);
                }
            }
        }
    })
}

/// The root of the work tree `cwd` is in (git lists untracked files under the current directory only).
fn toplevel(cwd: &Path) -> PathBuf {
    let out = git(cwd, &["rev-parse", "--show-toplevel"], b"");
    let root = String::from_utf8_lossy(&out).trim().to_string();
    if root.is_empty() { cwd.to_path_buf() } else { PathBuf::from(root) }
}

/// A file of the git directory of the repository `cwd` belongs to, `None` outside a repository.
fn git_file(cwd: &Path, name: &str) -> Option<PathBuf> {
    let out = git(cwd, &["rev-parse", "--git-path", name], b"");
    let path = String::from_utf8_lossy(&out).trim().to_string();
    (!path.is_empty()).then(|| cwd.join(path))
}

fn marker_path(cwd: &Path) -> Option<PathBuf> {
    git_file(cwd, "claude-code-review.json")
}

fn record(cwd: &Path) -> std::io::Result<()> {
    let Some(path) = marker_path(cwd) else { return Ok(()) };
    let snap = snapshot(cwd);
    std::fs::write(path, serde_json::json!({"head": snap.head, "hash": snap.hash, "files": snap.files}).to_string())
}

/// Why the commit must wait for a review, or `None` when the review covers it.
fn refusal(cwd: &Path) -> Option<&'static str> {
    let snap = snapshot(cwd);
    let path = marker_path(cwd)?;
    // the waiver is spent by a commit, not by a command that may not commit (`--dry-run`, a refusal of another hook
    // or of git): it is stamped with `HEAD` when it lets a commit through and dropped once `HEAD` moved
    let stamp = if snap.head.is_empty() { NO_HEAD } else { snap.head.as_str() };
    let waiver = git_file(cwd, WAIVER).filter(|w| w.exists());
    let used = waiver.as_ref().map(|w| std::fs::read_to_string(w).unwrap_or_default().trim().to_string());
    if let (Some(waiver), Some(used)) = (&waiver, &used) {
        if !used.is_empty() && used != stamp {
            std::fs::remove_file(waiver).ok();
        }
    }
    if snap.files.is_empty() || snap.files.iter().all(|f| f.starts_with(ISSUES_DIR)) {
        return None;
    }
    if let (Some(waiver), Some(used)) = (&waiver, &used) {
        if used == stamp {
            return None;
        }
        if used.is_empty() {
            // fail closed: a waiver that cannot be stamped could never be spent
            if std::fs::write(waiver, stamp).is_err() {
                return Some("the waiver could not be stamped");
            }
            return None;
        }
    }
    let Some(marker) = std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else {
        return Some("no code review has run in this project");
    };
    if marker["head"].as_str() != Some(snap.head.as_str()) {
        return Some("the last code review predates the last commit");
    }
    if marker["hash"].as_str() == Some(snap.hash.as_str()) {
        return None;
    }
    // a fix made after the review touches files the review already saw; a file it never saw is refused
    let reviewed: Vec<&str> = marker["files"].as_array().into_iter().flatten().filter_map(|f| f.as_str()).collect();
    if snap.files.iter().all(|f| reviewed.contains(&f.as_str())) {
        return None;
    }
    Some("the diff holds files the last code review never saw")
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let event: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let project = std::env::var("CLAUDE_PROJECT_DIR").ok().map(PathBuf::from);
    let cwd = event["cwd"].as_str().map(PathBuf::from).or(project).unwrap_or_else(|| ".".into());
    let shell_dir = cwd;
    let cwd = toplevel(&shell_dir);
    match mode.as_str() {
        "record" => {
            let skill = event["tool_input"]["skill"].as_str().unwrap_or_default();
            let succeeded = event["tool_response"]["success"].as_bool() == Some(true);
            if skill.rsplit(':').next() == Some(REVIEW_SKILL) && succeeded {
                if let Err(e) = record(&cwd) {
                    eprintln!("Could not record the code review marker: {e}");
                    std::process::exit(1);
                }
            }
        }
        "check" => {
            let Some(command) = event["tool_input"]["command"].as_str() else { return };
            if command.contains(WAIVER) {
                eprintln!("Refused: only the user waives the code review, by creating {WAIVER} in the git directory.");
                std::process::exit(2);
            }
            let Some(dirs) = commit_dirs(command) else { return };
            let mut roots: Vec<PathBuf> = dirs.into_iter().map(|d| d.map_or(cwd.clone(), |d| toplevel(&shell_dir.join(d)))).collect();
            roots.dedup();
            for cwd in roots {
                if let Some(why) = refusal(&cwd) {
                    eprintln!("Refused: {why}.");
                    eprintln!("Run /code-review on the current diff with the Skill tool (issue-workflow, step 4.5), fix what it finds, then commit.");
                    std::process::exit(2);
                }
            }
        }
        _ => {}
    }
}
