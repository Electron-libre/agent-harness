#!/usr/bin/env rust-script
//! Print one tab-separated row of numbers per session of the Claude Code transcripts (`specs/evaluation.md`): cost,
//! user interventions, and what may explain a difference (project, issue, harness revision, model, version). The
//! sub-agent transcripts count with their session. No text of a message is printed, so the rows can be archived.
//! Usage: `rust-script session_metrics.rs [projects_dir]` (default `~/.claude/projects`).
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! chrono = "0.4"
//! ```

#[path = "transcripts.rs"]
mod transcripts;

use chrono::{DateTime, FixedOffset};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use transcripts::{projects, projects_root, read_lines, sessions, subagents, typed_text};

const HEADER: &str = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields";

/// The line types seen in the transcripts; any other is counted in `unknown_lines`, since the format moves.
const KNOWN_TYPES: &[&str] = &[
    "user", "assistant", "attachment", "system", "summary", "ai-title", "custom-title", "last-prompt", "queue-operation", "api-request",
    "api-request-blob", "api-request-shape", "atis-latch", "mode", "cost-state", "file-history-snapshot",
];

/// The typed messages that only validate a stop, compared trimmed, lower-cased, trailing `.` and `!` removed.
const VALIDATIONS: &[&str] = &["ok", "okay", "yes", "y", "oui", "go", "continue", "lgtm", "vas-y", "c'est bon"];

/// The lines of the conversation: the only ones that must carry a `timestamp`.
const CONVERSATION_TYPES: &[&str] = &["user", "assistant", "attachment", "system"];

/// Branches that name no issue.
const DEFAULT_BRANCHES: &[&str] = &["main", "master", "HEAD"];

#[derive(Default)]
struct Session {
    cwd: Option<String>,
    issues: Vec<String>,
    models: Vec<String>,
    versions: Vec<String>,
    start: Option<(DateTime<FixedOffset>, String)>,
    end: Option<DateTime<FixedOffset>>,
    messages: HashSet<String>,
    turns: u64,
    tool_calls: u64,
    input: u64,
    output: u64,
    thinking: u64,
    cache_read: u64,
    cache_creation: u64,
    hooks: BTreeMap<String, u64>,
    skills: BTreeMap<String, u64>,
    validations: u64,
    others: u64,
    unreadable: u64,
    unknown: u64,
    missing: u64,
}

fn push_distinct(list: &mut Vec<String>, value: Option<&str>) {
    if let Some(v) = value.filter(|v| !v.is_empty() && !list.iter().any(|k| k == v)) {
        list.push(v.to_string());
    }
}

/// A plain validation, or the answers to the questions of a stop (`1 a, 2 c`).
fn is_validation(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    let t = t.trim_end_matches(['.', '!']).trim();
    let answer = |item: &str| {
        let item: String = item.chars().filter(|c| !c.is_whitespace()).collect();
        let digits = item.trim_end_matches(|c: char| c.is_ascii_lowercase());
        let letters = item.len() - digits.len();
        letters == 1 && !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
    };
    VALIDATIONS.contains(&t) || (!t.is_empty() && t.split(',').all(answer))
}

/// The text of a tool result: its string content, or its text parts joined.
fn result_text(block: &Value) -> String {
    match &block["content"] {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

/// The hook a refusal names: the stem of the first `.rs` file its text cites, `unknown` when none.
fn hook_name(text: &str) -> String {
    let Some(end) = text.find(".rs") else { return "unknown".into() };
    let stem: String = text[..end].chars().rev().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
    if stem.is_empty() { "unknown".into() } else { stem.chars().rev().collect() }
}

impl Session {
    fn read(&mut self, path: &Path, main: bool) {
        let (lines, unreadable) = match read_lines(path) {
            Ok(read) => read,
            Err(e) => {
                eprintln!("cannot read {}: {e}", path.display());
                return;
            }
        };
        self.unreadable += unreadable as u64;
        for line in &lines {
            self.line(line, main);
        }
    }

    fn line(&mut self, line: &Value, main: bool) {
        let kind = line["type"].as_str().unwrap_or_default();
        if !KNOWN_TYPES.contains(&kind) {
            self.unknown += 1;
        }
        match line["timestamp"].as_str().and_then(|t| DateTime::parse_from_rfc3339(t).ok().map(|d| (d, t))) {
            Some((at, text)) => {
                if self.start.as_ref().is_none_or(|(s, _)| at < *s) {
                    self.start = Some((at, text.to_string()));
                }
                if self.end.is_none_or(|e| at > e) {
                    self.end = Some(at);
                }
            }
            None if CONVERSATION_TYPES.contains(&kind) => self.missing += 1,
            None => {}
        }
        if main && self.cwd.is_none() {
            self.cwd = line["cwd"].as_str().map(str::to_string);
        }
        push_distinct(&mut self.versions, line["version"].as_str());
        push_distinct(&mut self.issues, line["gitBranch"].as_str().filter(|b| !DEFAULT_BRANCHES.contains(b)));
        match kind {
            "assistant" => self.assistant(&line["message"]),
            "user" => self.user(line, main),
            _ => {}
        }
    }

    fn assistant(&mut self, message: &Value) {
        let blocks = message["content"].as_array().map(Vec::as_slice).unwrap_or_default();
        for block in blocks.iter().filter(|b| b["type"] == "tool_use") {
            self.tool_calls += 1;
            if block["name"] == "Skill" {
                *self.skills.entry(block["input"]["skill"].as_str().unwrap_or("unknown").to_string()).or_default() += 1;
            }
        }
        let usage = &message["usage"];
        let Some(id) = message["id"].as_str().filter(|_| usage.is_object()) else {
            self.missing += 1;
            return;
        };
        if !self.messages.insert(id.to_string()) {
            return;
        }
        self.turns += 1;
        push_distinct(&mut self.models, message["model"].as_str());
        let tokens = |key: &str| usage[key].as_u64().unwrap_or(0);
        self.input += tokens("input_tokens");
        self.output += tokens("output_tokens");
        self.cache_read += tokens("cache_read_input_tokens");
        self.cache_creation += tokens("cache_creation_input_tokens");
        self.thinking += usage["output_tokens_details"]["thinking_tokens"].as_u64().unwrap_or(0);
    }

    fn user(&mut self, line: &Value, main: bool) {
        match typed_text(line).filter(|_| main) {
            Some(text) if is_validation(&text) => self.validations += 1,
            Some(_) => self.others += 1,
            None => {}
        }
        let blocks = line["message"]["content"].as_array().map(Vec::as_slice).unwrap_or_default();
        for block in blocks.iter().filter(|b| b["type"] == "tool_result" && b["is_error"].as_bool() == Some(true)) {
            let text = result_text(block);
            if text.starts_with("PreToolUse:") && text.contains(" hook error:") {
                *self.hooks.entry(hook_name(&text)).or_default() += 1;
            }
        }
    }

    fn row(&self, id: &str, project: &str, revision: &str) -> String {
        let pairs = |map: &BTreeMap<String, u64>| map.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(";");
        let (start, duration) = match (&self.start, self.end) {
            (Some((s, text)), Some(e)) => (text.clone(), (e - *s).num_seconds().to_string()),
            _ => (String::new(), String::new()),
        };
        [
            id.to_string(),
            project.to_string(),
            self.issues.join(","),
            revision.to_string(),
            self.models.join(","),
            self.versions.join(","),
            start,
            duration,
            self.turns.to_string(),
            self.tool_calls.to_string(),
            self.input.to_string(),
            self.output.to_string(),
            self.thinking.to_string(),
            self.cache_read.to_string(),
            self.cache_creation.to_string(),
            pairs(&self.hooks),
            pairs(&self.skills),
            self.validations.to_string(),
            self.others.to_string(),
            self.unreadable.to_string(),
            self.unknown.to_string(),
            self.missing.to_string(),
        ]
        .join("\t")
    }
}

/// The name of the repository holding `cwd` (a linked worktree counts with its main repository), else the last
/// component of `cwd`.
fn project_name(cwd: &str) -> String {
    let out = Command::new("git").args(["-C", cwd, "rev-parse", "--path-format=absolute", "--git-common-dir"]).output();
    let common = out.ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let repo = common.as_deref().and_then(|c| Path::new(c).parent()).and_then(Path::file_name);
    repo.or_else(|| Path::new(cwd).file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(flag) = args.iter().find(|a| a.starts_with('-')) {
        eprintln!("unknown option {flag}\nusage: session_metrics.rs [projects_dir]");
        std::process::exit(2);
    }
    let root = match projects_root(args.first()) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let mut names: HashMap<String, String> = HashMap::new();
    println!("{HEADER}");
    for project in projects(&root) {
        for path in sessions(&project) {
            let mut session = Session::default();
            session.read(&path, true);
            for sub in subagents(&path) {
                session.read(&sub, false);
            }
            let id = path.file_stem().unwrap().to_string_lossy().into_owned();
            let cwd = session.cwd.clone().unwrap_or_default();
            let name = names.entry(cwd.clone()).or_insert_with(|| if cwd.is_empty() { String::new() } else { project_name(&cwd) });
            println!("{}", session.row(&id, name, ""));
        }
    }
}
