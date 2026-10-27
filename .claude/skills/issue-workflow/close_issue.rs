#!/usr/bin/env rust-script
//! Move a finished issue file into `issues/done/` (only run after the user agreed).
//! Uses `git mv` when the file is tracked, a plain rename otherwise. Refuses to overwrite.
//! Then rewrites every `issues/foo.md` mentioned in the other text files of the repo to
//! `issues/done/foo.md`, so no document keeps pointing at the old path; the issue worktrees nested under
//! `.claude/worktrees/` keep their own files.
//! Refuses to run in a linked git worktree, and to close an issue of one named by path from elsewhere: the
//! queue is edited at merge time, on the main worktree.
//! Usage: `just close-issue issues/foo.md`; `just close-issue --relink` fixes the references to
//! every issue already in `issues/done/` (run it once to repair old dangling paths).
//! ```cargo
//! [dependencies]
//! toml = "0.8"
//! ```

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

const SKIPPED_DIRS: [&str; 3] = [".git", "target", "node_modules"];

/// Strings of `key = ["a", "b"]` in the `[project]` table of `.claude/harness.toml`; empty when absent.
fn config_list(root: &std::path::Path, key: &str) -> Vec<String> {
    let text = std::fs::read_to_string(root.join(".claude/harness.toml")).unwrap_or_default();
    let table: toml::Table = text.parse().unwrap_or_default();
    let list = table.get("project").and_then(|p| p.get(key)).or_else(|| table.get(key));
    list.and_then(toml::Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn is_text_file(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == "justfile")
        || path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| ["md", "rs", "toml", "yaml", "yml", "txt", "json"].contains(&e))
}

/// The text files under `dir`, except in the `skipped` directory names and the `worktrees` directory (the issue
/// worktrees nested in the main one, ADR 0008), matched by full path.
fn text_files(dir: &Path, out: &mut Vec<std::path::PathBuf>, skipped: &[String], worktrees: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path != worktrees && !path.file_name().and_then(|n| n.to_str()).is_some_and(|n| skipped.iter().any(|s| s == n)) {
                text_files(&path, out, skipped, worktrees);
            }
        } else if is_text_file(&path) {
            out.push(path);
        }
    }
}

/// Insert `done/` in `issues/<stem>.md` for the given stems, only where `issues/` starts a path
/// (not `my_issues/` nor `docs/issues/`).
fn relink_text(text: &str, stems: &HashSet<String>) -> String {
    const MARK: &str = "issues/";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(MARK) {
        out.push_str(&rest[..i + MARK.len()]);
        let boundary = out[..out.len() - MARK.len()]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || "`\"'([<{=,:".contains(c));
        rest = &rest[i + MARK.len()..];
        let name: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
        if boundary && rest[name.len()..].starts_with(".md") && stems.contains(&name) {
            out.push_str("done/");
        }
    }
    out.push_str(rest);
    out
}

/// Rewrite the references to the given closed issues in every text file under `root`.
fn relink(root: &Path, stems: &HashSet<String>) {
    let mut files = Vec::new();
    let mut skipped: Vec<String> = SKIPPED_DIRS.iter().map(|s| (*s).to_string()).collect();
    skipped.extend(config_list(root, "skipped_dirs"));
    text_files(root, &mut files, &skipped, &root.join(".claude/worktrees"));
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        let updated = relink_text(&text, stems);
        if updated == text {
            continue;
        }
        match std::fs::write(&file, updated) {
            Ok(()) => println!("updated references in {}", file.display()),
            Err(e) => eprintln!("could not update {}: {e}", file.display()),
        }
    }
}

/// Root of the git repository containing `dir`; the references are only rewritten inside it.
fn repo_root(dir: &Path) -> Option<std::path::PathBuf> {
    let out = Command::new("git").args(["rev-parse", "--show-toplevel"]).current_dir(dir).output().ok()?;
    out.status.success().then(|| std::path::PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/// `Some(message)` when `dir` is in a git worktree other than the main one.
fn linked_worktree(dir: &Path) -> Option<String> {
    let top = repo_root(dir)?;
    let out = Command::new("git").args(["worktree", "list", "--porcelain"]).current_dir(dir).output().ok()?;
    let list = String::from_utf8_lossy(&out.stdout);
    if list.split("\n\n").next()?.lines().any(|l| l == "bare") {
        return None;
    }
    let main = list.lines().find_map(|l| l.strip_prefix("worktree "))?;
    let same = std::fs::canonicalize(main).ok()? == std::fs::canonicalize(&top).ok()?;
    (!same).then(|| format!("{} is a linked worktree: merge its branch, then close the issue in the main worktree {main}", top.display()))
}

/// Exit with an error when `dir` is in a linked worktree (PAR-8).
fn refuse_linked_worktree(dir: &Path) {
    if let Some(message) = linked_worktree(dir) {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn main() {
    let Some(arg) = std::env::args().nth(1) else {
        eprintln!("usage: close_issue.rs issues/<name>.md | --relink");
        std::process::exit(2);
    };
    if arg == "--relink" {
        refuse_linked_worktree(Path::new("."));
        let stems: HashSet<String> = std::fs::read_dir("issues/done")
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_string))
            .filter(|s| s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'))
            .collect();
        match repo_root(Path::new(".")) {
            Some(root) => relink(&root, &stems),
            None => {
                eprintln!("not inside a git repository");
                std::process::exit(1);
            }
        }
        return;
    }
    let issue = Path::new(&arg);
    let (Some(dir), Some(name)) = (issue.parent(), issue.file_name()) else {
        eprintln!("not an issue file: {arg}");
        std::process::exit(2);
    };
    if !issue.is_file() || dir.file_name().is_none_or(|d| d != "issues") {
        eprintln!("{arg} is not an issue file directly under an `issues/` directory");
        std::process::exit(1);
    }
    refuse_linked_worktree(Path::new("."));
    refuse_linked_worktree(dir);
    let done = dir.join("done");
    let target = done.join(name);
    if target.exists() {
        eprintln!("{} already exists", target.display());
        std::process::exit(1);
    }
    std::fs::create_dir_all(&done).expect("done dir is creatable");
    // Run git in the issue's directory with paths relative to it, so the caller's directory does not matter.
    let tracked = Command::new("git")
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(name)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success());
    let moved = if tracked {
        Command::new("git").arg("mv").arg(name).arg(Path::new("done").join(name)).current_dir(dir).status().is_ok_and(|s| s.success())
    } else {
        std::fs::rename(issue, &target).is_ok()
    };
    if !moved {
        eprintln!("could not move {arg}");
        std::process::exit(1);
    }
    let how = if tracked { "git mv" } else { "plain rename, not tracked by git" };
    println!("moved {} -> {} ({how})", issue.display(), target.display());
    let stem = Path::new(name).file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    // Drop the issue from the queue so `next_issue.rs` never offers a closed one.
    let queue = dir.join("queue.md");
    if let Ok(text) = std::fs::read_to_string(&queue) {
        let mut kept: Vec<&str> = text
            .lines()
            .filter(|l| !l.trim().starts_with(&format!("- [{stem}](")))
            .collect();
        if kept.len() != text.lines().count() {
            // Closing the last entry would leave the blank line under the header: end with one newline.
            while kept.last().is_some_and(|l| l.trim().is_empty()) {
                kept.pop();
            }
            let rewritten = if kept.is_empty() { String::new() } else { kept.join("\n") + "\n" };
            std::fs::write(&queue, rewritten).expect("queue is writable");
            println!("removed {stem} from {}", queue.display());
        }
    }
    // Last, so that a failure here never leaves the queue offering a closed issue.
    match repo_root(dir) {
        Some(root) => relink(&root, &HashSet::from([stem.to_string()])),
        None => eprintln!("not inside a git repository: references to {stem} were not updated"),
    }
}
