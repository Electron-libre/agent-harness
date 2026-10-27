#!/usr/bin/env rust-script
//! PostToolUse hook (Edit|Write): run `cargo fmt --all` after a Rust file was edited, in the work tree of the event's
//! `cwd` (an agent that entered an issue worktree formats that one), else in `CLAUDE_PROJECT_DIR`.
//!
//! Declare: PostToolUse Edit|Write
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```

use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

/// The top-level of the work tree holding the event's `cwd` (the `cwd` itself outside a repository), else
/// `CLAUDE_PROJECT_DIR`, else the current directory.
fn work_tree(event: &serde_json::Value) -> PathBuf {
    let Some(cwd) = event["cwd"].as_str().map(PathBuf::from) else {
        return std::env::var("CLAUDE_PROJECT_DIR").unwrap_or_else(|_| ".".into()).into();
    };
    let out = Command::new("git").args(["rev-parse", "--show-toplevel"]).current_dir(&cwd).output();
    let root = out.ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    root.filter(|r| !r.is_empty()).map_or(cwd, PathBuf::from)
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let event: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let is_rust = event["tool_input"]["file_path"]
        .as_str()
        .is_some_and(|p| p.ends_with(".rs"));
    if !is_rust {
        return;
    }
    let dir = work_tree(&event);
    // Formatting is best-effort: a syntax error mid-edit must not block the agent.
    let _ = Command::new("cargo").args(["fmt", "--all"]).current_dir(dir).status();
}
