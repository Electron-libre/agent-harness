//! Behavior test for the Stop hook `cardinal tool language-hint` (`specs/language-hint.md`): a temp repository with a
//! `user_language`, a `Stop` event on stdin, the exit code and the hint on stderr.

mod common;

use common::{output, put, Run};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

const FRENCH: &str = "Le livrable est fait : les tests passent et la revue ne trouve rien, donc je committe puis je passe au suivant dans la foulée.";
const ENGLISH: &str = "The deliverable is done: the tests pass and the review finds nothing, so I commit it and move on to the next one with the plan.";

/// A git repository whose `.claude/harness.toml` names `language` (none when `None`).
fn repo(dir: &Path, language: Option<&str>) {
    let ok = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok, "git init failed");
    let key = language
        .map(|l| format!("user_language = \"{l}\"\n"))
        .unwrap_or_default();
    put(
        dir,
        ".claude/harness.toml",
        &format!("[harness]\nsource = \"x\"\n\n[project]\n{key}"),
    );
}

fn hook(project: &Path, event: &Value) -> Run {
    hook_raw(project, &event.to_string())
}

fn hook_raw(project: &Path, stdin: &str) -> Run {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cardinal"));
    cmd.args(["tool", "language-hint"])
        .env("CLAUDE_PROJECT_DIR", project);
    output(cmd, stdin)
}

fn stop(cwd: &Path, message: &str) -> Value {
    json!({"hook_event_name": "Stop", "stop_hook_active": false, "cwd": cwd, "last_assistant_message": message})
}

fn silent_pass(run: &Run) -> bool {
    run.code == Some(0) && run.stderr.is_empty() && run.stdout.is_empty()
}

// spec: LNG-5
#[test]
fn a_reply_in_the_other_language_gets_one_line_naming_the_language() {
    let dir = tempdir().unwrap();
    repo(dir.path(), Some("French"));
    let run = hook(dir.path(), &stop(dir.path(), ENGLISH));
    assert_eq!(run.code, Some(2), "{run:?}");
    assert_eq!(run.stderr.lines().count(), 1, "{run:?}");
    assert!(run.stderr.contains("French"), "{run:?}");
    assert!(
        !run.stderr.contains("deliverable"),
        "quotes the reply: {run:?}"
    );

    let dir = tempdir().unwrap();
    repo(dir.path(), Some("English"));
    let run = hook(dir.path(), &stop(dir.path(), FRENCH));
    assert_eq!(run.code, Some(2), "{run:?}");
    assert!(run.stderr.contains("English"), "{run:?}");
}

// spec: LNG-1
#[test]
fn the_hook_lets_go_once_it_sent_the_agent_back() {
    let dir = tempdir().unwrap();
    repo(dir.path(), Some("French"));
    let mut event = stop(dir.path(), ENGLISH);
    event["stop_hook_active"] = json!(true);
    assert!(silent_pass(&hook(dir.path(), &event)));
}

// spec: LNG-4, LNG-6
#[test]
fn code_short_replies_and_replies_in_the_language_pass() {
    let dir = tempdir().unwrap();
    repo(dir.path(), Some("French"));
    for message in [
        FRENCH.to_string(),
        "The tests pass, so I commit it and move on.".to_string(),
        format!("Voici le diff :\n```\n{ENGLISH}\n```\net `{ENGLISH}` fini."),
        format!("Voici la sortie :\n   ~~~\n{ENGLISH}\n   ~~~\nfini."),
    ] {
        let run = hook(dir.path(), &stop(dir.path(), &message));
        assert!(silent_pass(&run), "{message}: {run:?}");
    }
}

// spec: LNG-6
#[test]
fn a_reply_that_cannot_be_judged_passes() {
    for language in [None, Some("German")] {
        let dir = tempdir().unwrap();
        repo(dir.path(), language);
        assert!(silent_pass(&hook(dir.path(), &stop(dir.path(), ENGLISH))));
    }
    let dir = tempdir().unwrap();
    repo(dir.path(), Some("French"));
    assert!(silent_pass(&hook_raw(dir.path(), "not json")));
    let mut event = stop(dir.path(), ENGLISH);
    event
        .as_object_mut()
        .unwrap()
        .remove("last_assistant_message");
    event["transcript_path"] = json!(dir.path().join("missing.jsonl"));
    assert!(silent_pass(&hook(dir.path(), &event)));
    put(dir.path(), ".claude/harness.toml", "[project\n");
    assert!(silent_pass(&hook(dir.path(), &stop(dir.path(), ENGLISH))));
}

// spec: LNG-2
#[test]
fn with_no_last_message_the_last_assistant_text_of_the_transcript_is_judged() {
    let dir = tempdir().unwrap();
    repo(dir.path(), Some("French"));
    let lines = [
        json!({"type": "user", "message": {"role": "user", "content": "SECRET"}}),
        json!({"type": "assistant", "message": {"id": "m1", "content": [{"type": "text", "text": FRENCH}]}}),
        json!({"type": "assistant", "message": {"id": "m2", "content": [{"type": "text", "text": ENGLISH}]}}),
        json!({"type": "assistant", "message": {"id": "m2", "content": [{"type": "tool_use", "id": "t", "name": "Bash", "input": {}}]}}),
        json!({"type": "user", "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t", "content": FRENCH}]}}),
    ];
    let text: String = lines.iter().map(|l| l.to_string() + "\n").collect();
    put(dir.path(), "t.jsonl", &text);
    let event = json!({"hook_event_name": "Stop", "cwd": dir.path(), "transcript_path": dir.path().join("t.jsonl")});
    let run = hook(dir.path(), &event);
    assert_eq!(run.code, Some(2), "{run:?}");

    let french_last: String = lines[..2].iter().map(|l| l.to_string() + "\n").collect();
    put(dir.path(), "t.jsonl", &french_last);
    assert!(silent_pass(&hook(dir.path(), &event)));

    // a new turn with no text yet: the English reply of the turn before is not judged again
    let new_turn: String = [&lines[2], &lines[0], &lines[3]]
        .iter()
        .map(|l| l.to_string() + "\n")
        .collect();
    put(dir.path(), "t.jsonl", &new_turn);
    let run = hook(dir.path(), &event);
    assert!(silent_pass(&run), "{run:?}");
}

// spec: LNG-3
#[test]
fn the_config_is_read_at_the_root_of_the_work_tree_of_the_cwd_else_of_the_project() {
    let dir = tempdir().unwrap();
    repo(dir.path(), Some("French"));
    put(dir.path(), "sub/x.txt", "");
    let other = tempdir().unwrap();
    repo(other.path(), Some("English"));
    // the cwd's work tree (French) wins over CLAUDE_PROJECT_DIR (English)
    let run = hook(other.path(), &stop(&dir.path().join("sub"), ENGLISH));
    assert_eq!(run.code, Some(2), "{run:?}");
    assert!(run.stderr.contains("French"), "{run:?}");
    // no cwd: CLAUDE_PROJECT_DIR
    let mut event = stop(dir.path(), FRENCH);
    event.as_object_mut().unwrap().remove("cwd");
    let run = hook(other.path(), &event);
    assert!(run.stderr.contains("English"), "{run:?}");
    // a cwd outside git: CLAUDE_PROJECT_DIR
    let outside = tempdir().unwrap();
    let run = hook(other.path(), &stop(outside.path(), FRENCH));
    assert!(run.stderr.contains("English"), "{run:?}");
    // a cwd in a work tree with no config (a nested clone): CLAUDE_PROJECT_DIR
    let bare = tempdir().unwrap();
    repo(bare.path(), None);
    std::fs::remove_file(bare.path().join(".claude/harness.toml")).unwrap();
    let run = hook(other.path(), &stop(bare.path(), FRENCH));
    assert!(run.stderr.contains("English"), "{run:?}");
}
