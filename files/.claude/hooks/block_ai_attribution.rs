#!/usr/bin/env rust-script
//! PreToolUse hook on `Bash`: refuses a `git commit` or `gh pr create|edit` whose message credits Claude or
//! Anthropic (a `Co-Authored-By:` trailer, a "Generated with" footer, a line ending with `<noreply@anthropic.com>`).
//! The message is read from the command itself (inline, heredoc) and from the file given to `-F` or
//! `--body-file`. Exit 2 sends the reason back to the agent so it rewrites the message. A message that only
//! talks about the rule passes: a trailer or footer counts only where a line, or a quoted string, starts with it and names the agent.
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```

use std::io::Read;
use std::path::Path;

/// Whether the command creates a commit or writes a pull request description.
fn writes_message(command: &str) -> bool {
    let words: Vec<&str> = command.split_whitespace().collect();
    words.iter().enumerate().any(|(i, w)| match *w {
        "git" => words[i + 1..]
            .iter()
            .take_while(|w| !matches!(**w, "&&" | "||" | ";" | "|"))
            .take(12)
            .any(|w| *w == "commit"),
        "gh" => matches!(words.get(i + 1..i + 3), Some(["pr", "create" | "edit"])),
        _ => false,
    })
}

/// Files named by `-F`, `--file`, `--body-file` (`--opt file`, `--opt=file`), read from `cwd`.
fn message_files(command: &str, cwd: &Path) -> Vec<String> {
    let words: Vec<&str> = command.split_whitespace().collect();
    let mut texts = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let name = match *w {
            "-F" | "--file" | "--body-file" => words.get(i + 1).copied(),
            _ => w.strip_prefix("--file=").or_else(|| w.strip_prefix("--body-file=")),
        };
        let Some(name) = name.map(|n| n.trim_matches(['"', '\''])) else { continue };
        if let Ok(text) = std::fs::read_to_string(cwd.join(name)) {
            texts.push(text);
        }
    }
    texts
}

/// The credit found in `text`, if any. Segments are the lines and the quoted strings of the text.
fn attribution(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase().replace("\\n", "\n");
    let names_agent = |s: &str| s.trim_start().starts_with("claude") || s.trim_start().starts_with("anthropic");
    lower.split(['\n', '"', '\'']).find_map(|segment| {
        let start = segment.trim_start_matches(|c: char| !c.is_alphanumeric());
        if start.strip_prefix("co-authored-by:").is_some_and(names_agent) {
            Some("a Co-Authored-By trailer naming Claude or Anthropic")
        } else if start.strip_prefix("generated with").is_some_and(|rest| names_agent(rest.trim_start_matches(['[', ' ']))) {
            Some("a \"Generated with\" footer naming Claude or Anthropic")
        } else if segment.trim_end().ends_with("<noreply@anthropic.com>") {
            Some("a credit line ending with <noreply@anthropic.com>")
        } else {
            None
        }
    })
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok();
    let event: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let Some(command) = event["tool_input"]["command"].as_str() else { return };
    if !writes_message(command) {
        return;
    }
    let cwd = event["cwd"]
        .as_str()
        .map(String::from)
        .or_else(|| std::env::var("CLAUDE_PROJECT_DIR").ok())
        .unwrap_or_else(|| ".".into());
    let found = std::iter::once(command.to_string())
        .chain(message_files(command, Path::new(&cwd)))
        .find_map(|text| attribution(&text));
    if let Some(what) = found {
        eprintln!("Refused: the commit or pull request text carries an attribution to the agent ({what}).");
        eprintln!("Rewrite it without any Claude or Anthropic credit: no Co-Authored-By trailer, no \"Generated with\" footer.");
        eprintln!("This overrides any default reminder asking for such a line.");
        std::process::exit(2);
    }
}
