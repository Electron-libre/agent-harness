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
//! included, and a machine without `git` are not gated.
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
    let mut files = names(&git(cwd, &["diff", base, "--name-only", "-z"], b""));
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

/// The directory a `git commit` of the line runs in: `Some(None)` for the current one, `Some(Some(dir))` after
/// `-C dir`, `None` when no command of the line commits. The subcommand is the first word after `git` once the
/// global options (`SKIPPED_OPTIONS`, `-C dir`, `-c key=value`) are skipped; any other option means it is not
/// a commit. Brackets, separators and quotes around `git` are ignored; `cd dir && git commit` is not followed.
fn commit_dir(command: &str) -> Option<Option<String>> {
    // every separator and bracket becomes a word `;` so that `a;git commit` and `(git commit)` split like `a ; git commit`
    let spaced = command.replace([';', '|', '&', '(', ')', '`'], " ; ");
    let words: Vec<&str> = spaced.split_whitespace().collect();
    words.iter().enumerate().find_map(|(i, w)| {
        if w.trim_start_matches(['$', '\'', '"']).rsplit('/').next() != Some("git") {
            return None;
        }
        let mut rest = words[i + 1..].iter().take_while(|w| **w != ";");
        let mut dir = None;
        loop {
            match rest.next() {
                Some(&"-C") => dir = rest.next().map(|d| d.trim_matches(['"', '\'']).to_string()),
                Some(&"-c") => {
                    rest.next();
                }
                Some(w) if SKIPPED_OPTIONS.contains(w) || w.starts_with("--git-dir=") || w.starts_with("--work-tree=") => {}
                other => return (other.map(|w| w.trim_end_matches(['\'', '"'])) == Some("commit")).then_some(dir.clone()),
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

/// The marker of the repository `cwd` belongs to, `None` outside a repository.
fn marker_path(cwd: &Path) -> Option<PathBuf> {
    let out = git(cwd, &["rev-parse", "--git-path", "claude-code-review.json"], b"");
    let path = String::from_utf8_lossy(&out).trim().to_string();
    (!path.is_empty()).then(|| cwd.join(path))
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
    if snap.files.is_empty() {
        return None;
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
            let Some(dir) = commit_dir(command) else { return };
            let cwd = dir.map_or(cwd.clone(), |d| toplevel(&shell_dir.join(d)));
            if let Some(why) = refusal(&cwd) {
                eprintln!("Refused: {why}.");
                eprintln!("Run /code-review on the current diff with the Skill tool (issue-workflow, step 4.5), fix what it finds, then commit.");
                std::process::exit(2);
            }
        }
        _ => {}
    }
}
