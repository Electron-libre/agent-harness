#!/usr/bin/env rust-script
//! Stop hook: if Rust files changed, clippy must be warning-free before the agent can finish.
//! Exit 2 sends clippy's diagnostics back to the agent so it fixes the code: one line each (file, line, column,
//! message), at most `MAX_DIAGNOSTICS`, since every line is billed to the agent. A passing run stamps the Rust state
//! in the git directory, and clippy does not run again until that state changes. When the agent already retried once
//! (`stop_hook_active`), let go to avoid an endless loop. Spec: `specs/clippy-gate.md`.
//!
//! Declare: Stop
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::process::Command;

const MAX_DIAGNOSTICS: usize = 40;

fn git(dir: &str, args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// A hash of what clippy sees of the Rust code: the commit, the diff of the tracked `.rs` files and the untracked ones.
fn rust_state(dir: &str) -> String {
    let mut hasher = DefaultHasher::new();
    git(dir, &["rev-parse", "HEAD"]).hash(&mut hasher);
    git(dir, &["diff", "HEAD", "--", "*.rs"]).hash(&mut hasher);
    for file in git(dir, &["ls-files", "--others", "--exclude-standard", "--", "*.rs"]).lines() {
        file.hash(&mut hasher);
        std::fs::read(std::path::Path::new(dir).join(file)).unwrap_or_default().hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// A diagnostic line of `--message-format=short`: `path:line:column: level: message`.
fn is_diagnostic(line: &str) -> bool {
    let mut parts = line.splitn(4, ':');
    let (path, line_no, column) = (parts.next(), parts.next(), parts.next());
    let number = |p: Option<&str>| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    path.is_some_and(|p| !p.trim().is_empty()) && number(line_no) && number(column) && parts.next().is_some()
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let event: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    if event["stop_hook_active"].as_bool().unwrap_or(false) {
        return;
    }
    let dir = std::env::var("CLAUDE_PROJECT_DIR").unwrap_or_else(|_| ".".into());

    let status = git(&dir, &["status", "--porcelain"]);
    if !status.lines().any(|l| l.ends_with(".rs")) {
        return;
    }
    let stamp = git(&dir, &["rev-parse", "--path-format=absolute", "--git-path", "claude-clippy-pass"]);
    let stamp = stamp.trim();
    let state = rust_state(&dir);
    if !stamp.is_empty() && std::fs::read_to_string(stamp).is_ok_and(|s| s == state) {
        return;
    }

    let clippy = Command::new("cargo")
        .args(["clippy", "--workspace", "--all-targets", "--message-format=short", "--", "-D", "warnings"])
        .current_dir(&dir)
        .output()
        .expect("cargo must be installed");
    if clippy.status.success() {
        if !stamp.is_empty() {
            let _ = std::fs::write(stamp, state);
        }
        return;
    }

    let stderr = String::from_utf8_lossy(&clippy.stderr);
    // the same diagnostic comes once per target (lib, lib test): keep the first
    let mut diagnostics: Vec<&str> = Vec::new();
    for line in stderr.lines().filter(|l| is_diagnostic(l)) {
        if !diagnostics.contains(&line) {
            diagnostics.push(line);
        }
    }
    eprintln!("cargo clippy reports problems; fix them (the project must stay warning-free):");
    if diagnostics.is_empty() {
        // no diagnostic to point at (a manifest or build error): show the end of the output
        let lines: Vec<&str> = stderr.lines().collect();
        eprintln!("{}", lines[lines.len().saturating_sub(MAX_DIAGNOSTICS)..].join("\n"));
    } else {
        eprintln!("{}", diagnostics.iter().take(MAX_DIAGNOSTICS).copied().collect::<Vec<_>>().join("\n"));
        if diagnostics.len() > MAX_DIAGNOSTICS {
            eprintln!("... {} more (run `cargo clippy` for the rest)", diagnostics.len() - MAX_DIAGNOSTICS);
        }
    }
    std::process::exit(2);
}
