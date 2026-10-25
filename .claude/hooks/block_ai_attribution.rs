#!/usr/bin/env rust-script
//! PreToolUse hook on `Bash`: refuses a `git commit` or `gh pr create|edit` whose message credits Claude or
//! Anthropic (a `Co-Authored-By:` trailer, a "Generated with" footer, a line ending with `<noreply@anthropic.com>`).
//! The message is read from the command itself (inline, heredoc) and from the file given to `-F` or
//! `--body-file`. Exit 2 sends the reason back to the agent so it rewrites the message. A message that only
//! talks about the rule passes: a trailer or footer counts only where a line, or a quoted string, starts with it and names the agent.
//!
//! Declare: PreToolUse Bash
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```

use std::io::Read;
use std::path::Path;

/// Options that take the next word as value, before the subcommand of `git` and of `gh`. `require_code_review.rs`
/// holds the same list for `git` (a hook is a single file, keep the two in step).
const GIT_VALUE_OPTIONS: [&str; 7] = ["-C", "-c", "--git-dir", "--work-tree", "--namespace", "--exec-path", "--super-prefix"];
const GH_VALUE_OPTIONS: [&str; 3] = ["-R", "--repo", "--hostname"];

/// Takes the value of an option from `words`: a quoted value may span several words.
fn skip_value<'a>(words: &mut impl Iterator<Item = &'a &'a str>) {
    let Some(first) = words.next() else { return };
    if let Some(quote) = first.chars().next().filter(|c| matches!(c, '"' | '\'')) {
        if first.len() == 1 || !first.ends_with(quote) {
            words.find(|w| w.ends_with(quote));
        }
    }
}

/// Whether the command creates a commit or writes a pull request description. The subcommand is the first word
/// after `git` or `gh` once their global options are skipped (the idea of `commit_dir` in `require_code_review.rs`):
/// every flag, and the value of the options of `GIT_VALUE_OPTIONS` and `GH_VALUE_OPTIONS` given as a separate word.
/// An unknown option that takes a value would hide the subcommand: add it to the list.
fn writes_message(command: &str) -> bool {
    // every separator and bracket becomes a word `;` so that `a;git commit` and `(git commit)` split like `a ; git commit`
    let spaced = command.replace([';', '|', '&', '(', ')', '`', '\n'], " ; ");
    let words: Vec<&str> = spaced.split_whitespace().collect();
    words.iter().enumerate().any(|(i, w)| {
        let mut rest = words[i + 1..].iter().take_while(|w| **w != ";");
        let (is_git, value_options): (bool, &[&str]) = match w.trim_start_matches(['$', '\'', '"']).rsplit('/').next() {
            Some("git") => (true, &GIT_VALUE_OPTIONS),
            Some("gh") => (false, &GH_VALUE_OPTIONS),
            _ => return false,
        };
        loop {
            match rest.next() {
                Some(w) if value_options.contains(w) => skip_value(&mut rest),
                Some(w) if w.starts_with('-') => {}
                Some(w) => {
                    let sub = w.trim_matches(['\'', '"']);
                    return if is_git { sub == "commit" } else { sub == "pr" && matches!(rest.next(), Some(&"create" | &"edit")) };
                }
                None => return false,
            }
        }
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
