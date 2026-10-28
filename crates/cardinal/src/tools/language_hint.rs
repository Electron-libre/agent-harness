//! `language-hint`, a Stop hook: when the last reply of the agent is not in the `user_language` of
//! `.claude/cardinal.toml`, as the test of rule `user-language` judges it (`language.rs`), exit 2 with a one-line hint
//! so the agent gives it again in that language. It lets go when the agent already retried (`stop_hook_active`) and
//! whenever the reply cannot be judged, so it never blocks for its own failure. Spec: `specs/language-hint.md`.

use super::language::Language;
use super::transcripts::{git, typed_text};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The configuration at the root of the work tree holding `dir` (`crate::config_of`), when there is one.
fn config_at(dir: &Path) -> Option<PathBuf> {
    let root = git(dir, &["rev-parse", "--show-toplevel"]).filter(|r| !r.is_empty())?;
    Some(crate::config_of(Path::new(&root))).filter(|c| c.exists())
}

/// The language of the config of the work tree of the event's `cwd`, else of `CLAUDE_PROJECT_DIR` (LNG-3).
fn user_language(event: &Value) -> Option<Language> {
    let project = std::env::var("CLAUDE_PROJECT_DIR").unwrap_or_else(|_| ".".into());
    let config = event["cwd"]
        .as_str()
        .and_then(|cwd| config_at(Path::new(cwd)))
        .or_else(|| config_at(Path::new(&project)))?;
    Language::of_config(&std::fs::read_to_string(config).ok()?.parse().ok()?)
}

/// The text blocks of the last assistant line holding text in the transcript, joined by a blank line; `None` when a
/// message the user typed comes after it (LNG-2).
fn last_text(transcript: &str) -> Option<String> {
    for line in transcript.lines().rev() {
        let Ok(line) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if typed_text(&line).is_some() {
            return None;
        }
        if line["type"] != "assistant" {
            continue;
        }
        let texts: Vec<&str> = line["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect();
        if !texts.is_empty() {
            return Some(texts.join("\n\n"));
        }
    }
    None
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
    let Some(language) = user_language(&event) else {
        return 0;
    };
    if reply(&event).and_then(|text| language.breached_by(&text)) != Some(true) {
        return 0;
    }
    let name = language.name;
    eprintln!(
        "Your last reply is not in {name}, the user_language of .claude/cardinal.toml: give it again in {name}."
    );
    2
}
