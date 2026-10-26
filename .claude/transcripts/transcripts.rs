//! Reading of the Claude Code transcripts, shared by the scripts that read them (`#[path]` include, ADR 0006):
//! the projects directory, its projects and sessions, the sub-agent transcripts of a session, the parsed lines and
//! the filter of the messages the user typed.
#![allow(dead_code)]

use serde_json::Value;
use std::path::{Path, PathBuf};

/// The entries of `dir`, sorted; empty when it cannot be read.
pub fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    paths.sort();
    paths
}

/// The projects directory: `arg` when given, else `~/.claude/projects`.
pub fn projects_root(arg: Option<&String>) -> Result<PathBuf, String> {
    match arg {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => std::env::var("HOME").map(|home| PathBuf::from(home).join(".claude/projects")).map_err(|_| "HOME is not set: pass the projects directory".to_string()),
    }
}

/// The project directories of the projects directory.
pub fn projects(root: &Path) -> Vec<PathBuf> {
    sorted_entries(root).into_iter().filter(|p| p.is_dir()).collect()
}

fn jsonl_files(dir: &Path) -> Vec<PathBuf> {
    sorted_entries(dir).into_iter().filter(|p| p.extension().is_some_and(|e| e == "jsonl")).collect()
}

/// The session transcripts of a project (`<session>.jsonl`), sub-agents excluded.
pub fn sessions(project: &Path) -> Vec<PathBuf> {
    jsonl_files(project)
}

/// The transcripts of the sub-agents of a session: `<session>/subagents/*.jsonl` next to `<session>.jsonl`.
pub fn subagents(session: &Path) -> Vec<PathBuf> {
    jsonl_files(&session.with_extension("").join("subagents"))
}

/// The parsed lines of a transcript and the count of non-empty lines that are not valid JSON.
pub fn read_lines(path: &Path) -> std::io::Result<(Vec<Value>, usize)> {
    let text = std::fs::read_to_string(path)?;
    let (mut lines, mut unreadable) = (Vec::new(), 0);
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<Value>(line) {
            Ok(value) => lines.push(value),
            Err(_) => unreadable += 1,
        }
    }
    Ok((lines, unreadable))
}

/// The text a user typed in a transcript line; `None` for tool results, meta and sidechain lines, injected text and other types.
pub fn typed_text(line: &Value) -> Option<String> {
    if line["type"] != "user" || line["isMeta"].as_bool() == Some(true) || line["isSidechain"].as_bool() == Some(true) {
        return None;
    }
    let content = &line["message"]["content"];
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts.iter().filter(|p| p["type"] == "text").filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join("\n"),
        _ => return None,
    };
    let t = text.trim();
    // Reminders, command echoes and the compaction summary are injected as user lines; a typed `<div> broken` is not.
    let injected = (t.starts_with('<') && t.ends_with('>')) || t.starts_with("This session is being continued");
    (!t.is_empty() && !injected).then_some(text)
}
