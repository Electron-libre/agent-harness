//! Hygiene test of the Claude Code settings of this repository: hooks point to real scripts or tools, the shared allowlist
//! covers the dev commands without opening arbitrary execution, the seeded allowlist holds no rule of one stack, and the
//! personal `settings.local.json` is git-ignored and free of dangerous rules.
//!

use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Commands that must never be pre-approved: arbitrary code, secrets, destructive or outward git.
const DANGEROUS: [&str; 10] = [
    "bash",
    "sh",
    "eval",
    "env",
    "filter-branch",
    "push",
    "pkill",
    "rm",
    "git add",
    "git commit",
];
/// Interpreters, matched with their version suffix (`python3`).
const INTERPRETERS: [&str; 4] = ["python", "node", "perl", "ruby"];
/// Dev commands the shared allowlist must cover so the agent doesn't stop to ask.
const REQUIRED: [&str; 7] = [
    "cargo build",
    "cargo test",
    "cargo clippy",
    "cargo fmt",
    "just ",
    "cardinal run",
    "git diff",
];

fn allow(settings: &Value) -> Vec<String> {
    settings["permissions"]["allow"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn dangerous(rule: &str) -> bool {
    let body = rule.strip_prefix("Bash(").unwrap_or(rule);
    let words = body.split(|c: char| c.is_whitespace() || c == ')');
    words
        .clone()
        .any(|w| INTERPRETERS.iter().any(|i| w.starts_with(i)))
        || DANGEROUS.iter().any(|d| {
            if d.contains(' ') {
                body.contains(d)
            } else {
                words.clone().any(|w| w == *d)
            }
        })
}

#[test]
fn settings_hygiene() {
    let root = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    let mut failures: Vec<String> = Vec::new();

    let shared: Value =
        serde_json::from_str(&fs::read_to_string(root.join(".claude/settings.json")).unwrap())
            .unwrap_or_else(|e| {
                failures.push(format!("settings.json is not valid JSON: {e}"));
                Value::Null
            });

    // hooks reference scripts that exist; a tool of `cardinal run` is checked by the tests of the harness source
    for event in shared["hooks"].as_object().into_iter().flatten() {
        for group in event.1.as_array().into_iter().flatten() {
            for hook in group["hooks"].as_array().into_iter().flatten() {
                let cmd = hook["command"].as_str().unwrap_or("");
                if cmd.starts_with("cardinal run ") {
                    continue;
                }
                let script = cmd
                    .split_whitespace()
                    .rev()
                    .find(|w| {
                        Path::new(w.trim_matches('"'))
                            .extension()
                            .is_some_and(|e| e == "rs")
                            && w.contains(".claude/")
                    })
                    .or(cmd.split_whitespace().last())
                    .unwrap_or("")
                    .replace("\"$CLAUDE_PROJECT_DIR\"", "");
                let script = script.trim_matches('"').trim_start_matches('/');
                if script.is_empty() || !root.join(script).exists() {
                    failures.push(format!(
                        "hook {} points to a missing script: {script}",
                        event.0
                    ));
                }
            }
        }
    }

    // shared allowlist: covers the dev commands, nothing dangerous, nothing machine specific
    let rules = allow(&shared);
    for needed in REQUIRED {
        if !rules.iter().any(|r| r.contains(needed)) {
            failures.push(format!("shared allowlist does not cover `{needed}`"));
        }
    }
    for rule in &rules {
        if dangerous(rule) {
            failures.push(format!("shared allowlist has a dangerous rule: {rule}"));
        }
        if rule.contains("/home/") || rule.contains("~/") {
            failures.push(format!(
                "shared allowlist has a machine specific path: {rule}"
            ));
        }
    }

    // seeded allowlist: stack-neutral, the stack rules come from the project
    let seed: Value =
        serde_json::from_str(&fs::read_to_string(root.join("seed/.claude/settings.json")).unwrap())
            .unwrap_or(Value::Null);
    for rule in allow(&seed).iter().filter(|r| r.starts_with("Bash(")) {
        if !(rule.starts_with("Bash(just") || rule.starts_with("Bash(git ")) {
            failures.push(format!(
                "seeded allowlist has a rule of one stack (only `just` and `git`): {rule}"
            ));
        }
    }

    // personal file: git-ignored, no dangerous rule left behind
    let ignored = Command::new("git")
        .args(["check-ignore", "-q", ".claude/settings.local.json"])
        .current_dir(&root)
        .status()
        .is_ok_and(|s| s.success());
    if !ignored {
        failures.push("`.claude/settings.local.json` is not git-ignored".into());
    }
    if let Ok(text) = fs::read_to_string(root.join(".claude/settings.local.json")) {
        let local: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        for rule in allow(&local).iter().filter(|r| dangerous(r)) {
            failures.push(format!(
                "settings.local.json has a dangerous rule (remove it): {rule}"
            ));
        }
    }

    for f in &failures {
        println!("FAIL - {f}");
    }
    if failures.is_empty() {
        println!("ok   - settings hygiene");
    }
    assert!(
        failures.is_empty(),
        "{} failure(s): see the FAIL lines",
        failures.len()
    );
}
