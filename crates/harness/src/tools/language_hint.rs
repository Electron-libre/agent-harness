//! `language-hint`, a Stop hook: when the last reply of the agent is not in the `user_language` of
//! `.claude/harness.toml`, as the test of rule `user-language` judges it (`language.rs`), exit 2 with a one-line hint
//! so the agent gives it again in that language. It lets go when the agent already retried (`stop_hook_active`) and
//! whenever the reply cannot be judged, so it never blocks for its own failure. Spec: `specs/language-hint.md`.

use super::language::Language;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The root of the work tree holding `dir`; `None` outside git.
fn work_tree(dir: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!root.is_empty()).then(|| root.into())
}

/// The `user_language` of the `.claude/harness.toml` of the work tree of the event's `cwd`, else of
/// `CLAUDE_PROJECT_DIR` (LNG-3).
fn user_language(event: &Value) -> Option<String> {
    let project = || {
        let dir = std::env::var("CLAUDE_PROJECT_DIR").unwrap_or_else(|_| ".".into());
        work_tree(Path::new(&dir)).unwrap_or_else(|| dir.into())
    };
    let root = event["cwd"]
        .as_str()
        .and_then(|cwd| work_tree(Path::new(cwd)))
        .unwrap_or_else(project);
    let text = std::fs::read_to_string(root.join(crate::CONFIG)).ok()?;
    let table: toml::Table = text.parse().ok()?;
    Some(
        table
            .get("project")?
            .get("user_language")?
            .as_str()?
            .to_string(),
    )
}

/// The text blocks of the last assistant line holding text in the transcript, joined by a blank line (LNG-2).
fn last_text(transcript: &str) -> Option<String> {
    transcript.lines().rev().find_map(|line| {
        let line: Value = serde_json::from_str(line).ok()?;
        if line["type"] != "assistant" {
            return None;
        }
        let texts: Vec<&str> = line["message"]["content"]
            .as_array()?
            .iter()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect();
        (!texts.is_empty()).then(|| texts.join("\n\n"))
    })
}

/// The reply to judge: the event's `last_assistant_message`, else the last text of its transcript.
fn reply(event: &Value) -> Option<String> {
    if let Some(message) = event["last_assistant_message"].as_str() {
        return Some(message.to_string());
    }
    last_text(&std::fs::read_to_string(event["transcript_path"].as_str()?).ok()?)
}

pub(super) fn main(_args: &[String]) -> u8 {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let Ok(event) = serde_json::from_str::<Value>(&input) else {
        return 0;
    };
    if event["stop_hook_active"].as_bool().unwrap_or(false) {
        return 0;
    }
    let Some(name) = user_language(&event) else {
        return 0;
    };
    let breach = Language::named(&name)
        .zip(reply(&event))
        .and_then(|(language, text)| language.breached_by(&text));
    if breach != Some(true) {
        return 0;
    }
    eprintln!(
        "Your last reply is not in {name}, the user_language of .claude/harness.toml: give it again in {name}."
    );
    2
}
