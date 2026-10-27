#!/usr/bin/env rust-script
//! PostToolUse hook (Edit|Write): run `cargo fmt --all` after a Rust file was edited, from the directory of that file,
//! so cargo formats the project holding it (an issue worktree, a crate in a subdirectory), not the session's.
//!
//! Declare: PostToolUse Edit|Write
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```

use std::io::Read;
use std::path::Path;
use std::process::Command;

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let event: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let Some(file) = event["tool_input"]["file_path"].as_str().filter(|p| p.ends_with(".rs")) else {
        return;
    };
    let dir = Path::new(file).parent().filter(|d| d.is_dir()).unwrap_or(Path::new("."));
    // Formatting is best-effort: a syntax error mid-edit must not block the agent.
    let _ = Command::new("cargo").args(["fmt", "--all"]).current_dir(dir).status();
}
