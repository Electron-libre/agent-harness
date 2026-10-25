#!/usr/bin/env rust-script
//! Print the next issue to work on: the first line of `issues/queue.md`, with its title and the rest of
//! the queue. Reports (stderr) issues missing from the queue and queue lines without a file.
//! An issue is taken, and skipped, when a branch named after its stem exists or a git worktree has it checked
//! out; the mark comes from git alone, so every worktree sees it.
//! Usage: `just next-issue` (from the repo root, or pass the `issues` directory as argument).

use std::path::Path;
use std::process::Command;

fn title(dir: &Path, stem: &str) -> String {
    std::fs::read_to_string(dir.join(format!("{stem}.md")))
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("# ").map(str::to_string)))
        .unwrap_or_else(|| "(no file)".to_string())
}

/// `- [stem](stem.md) - why` gives `(stem, why)`; any other line (title, prose) is not an entry.
fn entry(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim().strip_prefix("- [")?;
    let (stem, rest) = rest.split_once("](")?;
    let why = rest.split_once(')').map_or("", |(_, w)| w.trim_start_matches(" - "));
    Some((stem, why))
}

fn git_lines(dir: &Path, args: &[&str]) -> Vec<String> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// The branches of the repository that holds `dir`, with the path of the worktree that has them checked out,
/// if any. The branch of the main worktree is left out: it is the trunk, not an issue. Empty outside a git
/// repository.
fn branches(dir: &Path) -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> =
        git_lines(dir, &["for-each-ref", "--format=%(refname:short)", "refs/heads"]).into_iter().map(|b| (b, None)).collect();
    let (mut path, mut first) = (String::new(), true);
    for line in git_lines(dir, &["worktree", "list", "--porcelain"]) {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = p.to_string();
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            if first {
                out.retain(|(name, _)| name != b);
            } else if let Some(entry) = out.iter_mut().find(|(name, _)| name == b) {
                entry.1 = Some(path.clone());
            }
            first = false;
        } else if line.is_empty() {
            first = false;
        }
    }
    out
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "issues".to_string());
    let dir = Path::new(&dir);
    let Ok(queue) = std::fs::read_to_string(dir.join("queue.md")) else {
        eprintln!("no {}/queue.md", dir.display());
        std::process::exit(2);
    };
    let entries: Vec<(&str, &str)> = queue.lines().filter_map(entry).collect();

    let mut drift = false;
    for (stem, _) in &entries {
        if !dir.join(format!("{stem}.md")).is_file() {
            eprintln!("queue line without a file: {stem}");
            drift = true;
        }
    }
    let mut unlisted: Vec<String> = std::fs::read_dir(dir)
        .map(|d| d.flatten().filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().into_owned())).collect())
        .unwrap_or_default();
    unlisted.retain(|s| s != "queue" && dir.join(format!("{s}.md")).is_file() && !entries.iter().any(|(e, _)| e == s));
    unlisted.sort();
    for s in &unlisted {
        eprintln!("issue missing from the queue: {s}");
        drift = true;
    }

    let branches = branches(dir);
    let taken = |stem: &str| branches.iter().find(|(b, _)| b == stem).map(|(_, path)| path.as_deref());
    let free: Vec<(usize, &(&str, &str))> = entries.iter().enumerate().filter(|(_, (s, _))| taken(s).is_none()).collect();

    match free.first() {
        None if entries.is_empty() => println!("The queue is empty."),
        None => println!("Every issue of the queue is taken."),
        Some((_, (stem, why))) => {
            println!("Next: issues/{stem}.md\n  {}\n  why here: {why}", title(dir, stem));
            for (i, (s, _)) in free.iter().skip(1) {
                println!("{:>3}. {s}", i + 1);
            }
        }
    }
    for (stem, _) in &entries {
        if let Some(path) = taken(stem) {
            println!("Taken: {stem}{}", path.map_or(String::new(), |p| format!(" (worktree {p})")));
        }
    }
    std::process::exit(i32::from(drift));
}
