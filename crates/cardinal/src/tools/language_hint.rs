//! `language-hint`, a Stop and `PostToolUse` hook: when the text of the agent is not in the `user_language` of
//! `.claude/cardinal.toml`, as the test of rule `user-language` judges it (`language.rs`), it hints at that language.
//! At Stop it judges the last reply and exits 2 with a one-line hint so the agent gives it again, letting go once the
//! agent retried (`stop_hook_active`). After a tool call it judges the text written before the call and adds a
//! one-line context, once per turn, so the next texts are in that language; the call never fails. It lets go whenever
//! the text cannot be judged, so it never blocks for its own failure. Spec: `specs/language-hint.md`.

use super::language::{is_hint, Language, HINT_MARK};
use super::transcripts::{git, typed_text, user_turn};
use serde_json::{json, Value};
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

/// The text blocks written before the `tool_use` block `id`, back to the previous tool result or typed message,
/// joined by a blank line; `None` when the call is not in the transcript or another call of the same message comes
/// between those texts and it (LNG-7).
fn text_before_call(lines: &[Value], id: &str) -> Option<String> {
    let mut texts = Vec::new();
    let mut found = false;
    for line in lines.iter().rev() {
        if line["type"] == "user" {
            if !found {
                continue;
            }
            let content = line["message"]["content"].as_array();
            let result = content
                .into_iter()
                .flatten()
                .any(|b| b["type"] == "tool_result");
            if result || user_turn(line) {
                break;
            }
            continue;
        }
        if line["type"] != "assistant" {
            continue;
        }
        let blocks = line["message"]["content"].as_array();
        for block in blocks.into_iter().flatten().rev() {
            match block["type"].as_str() {
                Some("tool_use") if !found => found = block["id"] == id,
                Some("tool_use") => return None,
                Some("text") if found => texts.extend(block["text"].as_str()),
                _ => {}
            }
        }
    }
    texts.reverse();
    (found && !texts.is_empty()).then(|| texts.join("\n\n"))
}

/// The `PostToolUse` mode: the text before the call, judged once per turn, gets an added context (LNG-7 to LNG-10).
fn after_call(event: &Value, language: Language) -> u8 {
    let (Some(path), Some(id)) = (
        event["transcript_path"].as_str(),
        event["tool_use_id"].as_str(),
    ) else {
        return 0;
    };
    let Ok(transcript) = std::fs::read_to_string(path) else {
        return 0;
    };
    // the lines of the turn only, read back from the end: a long session holds megabytes of earlier turns, and a
    // long turn megabytes of tool results (which only mark where the text before a call starts) and of other lines
    let mut lines: Vec<Value> = Vec::new();
    for line in transcript.lines().rev() {
        if line.contains(r#""type":"tool_result""#) {
            lines.push(json!({"type": "user", "message": {"content": [{"type": "tool_result"}]}}));
            continue;
        }
        let own = [r#""type":"user""#, r#""type":"assistant""#]
            .iter()
            .any(|t| line.contains(t));
        if !own && !line.contains(HINT_MARK) {
            continue;
        }
        let Ok(line) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if is_hint(&line) {
            return 0;
        }
        if user_turn(&line) {
            break;
        }
        lines.push(line);
    }
    lines.reverse();
    if text_before_call(&lines, id).and_then(|text| language.breached_by(&text)) != Some(true) {
        return 0;
    }
    let name = language.name;
    let hint = format!(
        "Your text before this tool call is not in {name}, {HINT_MARK}: write your next texts to the user in {name}."
    );
    let output = json!({
        "hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": hint}
    });
    println!("{output}");
    0
}

pub(super) fn main(_args: &[String]) -> u8 {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let Ok(event) = serde_json::from_str::<Value>(&input) else {
        return 0;
    };
    if event["hook_event_name"] == "PostToolUse" {
        return user_language(&event).map_or(0, |language| after_call(&event, language));
    }
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
    eprintln!("Your last reply is not in {name}, {HINT_MARK}: give it again in {name}.");
    2
}
