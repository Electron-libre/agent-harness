#!/usr/bin/env rust-script
//! List where the agent broke a rule of the harness in the Claude Code transcripts (`specs/rule-breaches.md`): one
//! tab-separated row per finding, `breach` when the act went through, `blocked` when a hook or a permission rule
//! refused it. With `--summary`, count them per rule and harness revision over the sessions where the rule applied,
//! so a rule broken often (a hook candidate) stands out from one that rarely applies. No text of a message is printed.
//! Usage: `rust-script rule_breaches.rs [projects_dir] [--summary]` (default `~/.claude/projects`).
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! chrono = "0.4"
//! toml = "0.8"
//! ```

// transcripts.rs sha256: 9107c69c3dc3b74218133c7637511f56e5c62072439efc3568203b8b79886d31
#[path = "transcripts.rs"]
mod transcripts;

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use transcripts::{committed_file, for_each_numbered_line, git, project_name, projects, projects_root, repository, result_text, revision, sessions, subagents, typed_text, Place};

const HEADER: &str = "session\tproject\trevision\trule\tkind\tlocation\tdetail";
const SUMMARY_HEADER: &str = "rule\trevision\tapplied\tbreached\tbreaches\tblocked";

/// The tools that write a file, with the input key of its path.
const EDIT_TOOLS: &[(&str, &str)] = &[("Edit", "file_path"), ("Write", "file_path"), ("NotebookEdit", "notebook_path")];

/// Where the files of the validation stop go before the plan is validated (`issue-workflow`, step 1).
const PLAN_DIRS: &[&str] = &["issues/", "specs/", "docs/adr/"];

struct Call {
    name: String,
    input: Value,
    location: String,
    /// Whether its result was an error; `None` when no result was read.
    error: Option<bool>,
}

impl Call {
    /// The words of a `Bash` command; empty for another tool.
    fn words(&self) -> Vec<String> {
        if self.name == "Bash" { shell_words(self.input["command"].as_str().unwrap_or_default()) } else { Vec::new() }
    }

    /// Whether it is a `Bash` call running `git commit`.
    fn commits(&self) -> bool {
        let words = self.words();
        words.iter().any(|w| w == "git") && words.iter().any(|w| w == "commit")
    }
}

enum Event {
    Call(usize),
    Typed,
    /// An error tool result, with the call it answers when known.
    Error { location: String, text: String, call: Option<usize> },
}

/// The events of one transcript file, in order.
#[derive(Default)]
struct Transcript {
    calls: Vec<Call>,
    events: Vec<Event>,
}

impl Transcript {
    fn read(path: &Path, name: &str, main: bool, place: &mut Place) -> Transcript {
        let mut t = Transcript::default();
        let mut ids: HashMap<String, usize> = HashMap::new();
        let read = for_each_numbered_line(path, |n, line| {
            place.observe(line, main);
            let location = format!("{name}:{n}");
            let blocks = line["message"]["content"].as_array().map(Vec::as_slice).unwrap_or_default();
            match line["type"].as_str() {
                Some("assistant") => {
                    for block in blocks.iter().filter(|b| b["type"] == "tool_use") {
                        ids.insert(block["id"].as_str().unwrap_or_default().to_string(), t.calls.len());
                        t.events.push(Event::Call(t.calls.len()));
                        t.calls.push(Call { name: block["name"].as_str().unwrap_or_default().to_string(), input: block["input"].clone(), location: location.clone(), error: None });
                    }
                }
                Some("user") => {
                    if main && typed_text(line).is_some() {
                        t.events.push(Event::Typed);
                    }
                    for block in blocks.iter().filter(|b| b["type"] == "tool_result") {
                        let error = block["is_error"].as_bool() == Some(true);
                        let call = block["tool_use_id"].as_str().and_then(|id| ids.get(id).copied());
                        if let Some(i) = call {
                            t.calls[i].error = Some(error);
                        }
                        if error {
                            t.events.push(Event::Error { location: location.clone(), text: result_text(block), call });
                        }
                    }
                }
                _ => {}
            }
        });
        if let Err(e) = read {
            eprintln!("cannot read {}: {e}", path.display());
        }
        t
    }
}

/// The words of a shell command that the shell itself reads: quoted strings and here-document bodies (commit
/// messages, mostly) are dropped, so a message that names `--no-verify` is not a use of it.
fn shell_words(command: &str) -> Vec<String> {
    let mut code = String::new();
    let mut terminator: Option<String> = None;
    for line in command.lines() {
        if let Some(end) = &terminator {
            if line.trim() == end {
                terminator = None;
            }
            continue;
        }
        code.push_str(line);
        code.push('\n');
        if let Some((_, rest)) = line.split_once("<<") {
            let word: String = rest.trim_start_matches('-').trim_start().chars().filter(|c| !matches!(c, '\'' | '"')).take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            terminator = (!word.is_empty()).then_some(word);
        }
    }
    let (mut words, mut word, mut quote) = (Vec::new(), String::new(), None);
    for c in code.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c.is_whitespace() || ";&|()<>`".contains(c) => words.extend((!word.is_empty()).then(|| std::mem::take(&mut word))),
            None => word.push(c),
        }
    }
    words.extend((!word.is_empty()).then_some(word));
    words
}

struct Finding {
    rule: &'static str,
    blocked: bool,
    location: String,
    detail: String,
}

/// What a session is checked against: its work tree, to make paths relative, and the managed paths then.
struct Context {
    worktree: PathBuf,
    managed: Option<HashSet<String>>,
}

impl Context {
    fn relative(&self, path: &str) -> Option<String> {
        Path::new(path).strip_prefix(&self.worktree).ok().map(|p| p.to_string_lossy().into_owned())
    }
}

#[derive(Default)]
struct Verdict {
    applied: BTreeSet<&'static str>,
    findings: Vec<Finding>,
}

impl Verdict {
    fn add(&mut self, rule: &'static str, blocked: bool, location: &str, detail: String) {
        self.applied.insert(rule);
        self.findings.push(Finding { rule, blocked, location: location.to_string(), detail });
    }

    /// The rules read from the tool calls and results of one transcript; `main` for the session's own one.
    fn check(&mut self, t: &Transcript, context: &Context, main: bool) {
        let mut flagged: HashSet<usize> = HashSet::new();
        let mut refused = false;
        let mut plan_proposed = false;
        for event in &t.events {
            match event {
                Event::Typed => {
                    refused = false;
                    plan_proposed = false;
                }
                Event::Call(i) => {
                    let call = &t.calls[*i];
                    let blocked = call.error == Some(true);
                    let before = self.findings.len();
                    match call.name.as_str() {
                        "Bash" => {
                            let words = call.words();
                            let has = |w: &str| words.iter().any(|x| x == w);
                            if has("git") && (has("commit") || has("push")) {
                                self.applied.insert("no-verify");
                                if has("--no-verify") {
                                    self.add("no-verify", blocked, &call.location, String::new());
                                }
                            }
                            if call.commits() {
                                self.applied.extend(["review-before-commit", "review-waiver"]);
                                if refused && call.error == Some(false) {
                                    self.add("review-before-commit", false, &call.location, String::new());
                                    refused = false;
                                }
                            }
                            if has("just") && has("waive-review") {
                                self.add("review-waiver", blocked, &call.location, String::new());
                            }
                        }
                        "Skill" if call.input["skill"].as_str().is_some_and(|s| s.ends_with("code-review")) => refused = false,
                        name => {
                            let key = EDIT_TOOLS.iter().find(|(tool, _)| *tool == name).map(|(_, key)| *key);
                            let path = key.and_then(|k| call.input[k].as_str()).unwrap_or_default();
                            if path.is_empty() {
                                continue;
                            }
                            if Path::new(path).file_name().is_some_and(|n| n == "claude-review-waived") {
                                self.add("review-waiver", blocked, &call.location, String::new());
                            }
                            let relative = context.relative(path);
                            if let Some(managed) = &context.managed {
                                self.applied.insert("managed-copy");
                                if let Some(rel) = relative.as_ref().filter(|r| managed.contains(*r)) {
                                    self.add("managed-copy", blocked, &call.location, rel.clone());
                                }
                            }
                            if main {
                                if let Some(rel) = relative.filter(|r| plan_proposed && !PLAN_DIRS.iter().any(|d| r.starts_with(d))) {
                                    self.add("validation-stop", false, &call.location, rel);
                                }
                                let text = [&call.input["new_string"], &call.input["content"]].into_iter().filter_map(Value::as_str).collect::<String>();
                                let issue = context.relative(path).is_some_and(|r| r.starts_with("issues/"));
                                if issue && text.contains("Plan: proposed") {
                                    plan_proposed = true;
                                    self.applied.insert("validation-stop");
                                }
                            }
                        }
                    }
                    if self.findings.len() > before {
                        flagged.insert(*i);
                    }
                }
                Event::Error { location, text, call } => {
                    if call.is_some_and(|i| flagged.contains(&i)) || !text.contains("Refused:") {
                        continue;
                    }
                    if text.contains("Refused: only the user waives") {
                        self.add("review-waiver", true, location, String::new());
                    } else if text.contains("code review") && call.is_some_and(|i| t.calls[i].commits()) {
                        self.add("review-before-commit", true, location, String::new());
                        refused = true;
                    }
                }
            }
        }
    }
}

/// The managed paths of `.claude/harness.lock` (`<hash>[+x]  <path>` lines) committed at `start`.
fn managed_paths(repo: &Path, start: &str) -> Option<HashSet<String>> {
    let lock = committed_file(repo, start, ".claude/harness.lock")?;
    Some(lock.lines().filter(|l| !l.starts_with('#')).filter_map(|l| l.split_whitespace().nth(1)).map(str::to_string).collect())
}

fn main() {
    const USAGE: &str = "usage: rule_breaches.rs [projects_dir] [--summary]";
    let (mut dir, mut summary) = (None, false);
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--summary" => summary = true,
            flag if flag.starts_with('-') => {
                eprintln!("unknown option {flag}\n{USAGE}");
                std::process::exit(2);
            }
            _ => dir = Some(arg),
        }
    }
    let root = match projects_root(dir.as_ref()) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let mut rows = Vec::new();
    // (rule, revision) -> [applied, breached, breaches, blocked]
    let mut counts: BTreeMap<(&str, String), [u64; 4]> = BTreeMap::new();
    for project in projects(&root) {
        for path in sessions(&project) {
            let mut place = Place::default();
            let id = path.file_stem().unwrap().to_string_lossy().into_owned();
            let main_name = path.file_name().unwrap().to_string_lossy().into_owned();
            let mut transcripts = vec![(Transcript::read(&path, &main_name, true, &mut place), true)];
            for sub in subagents(&path) {
                let name = format!("subagents/{}", sub.file_name().unwrap().to_string_lossy());
                transcripts.push((Transcript::read(&sub, &name, false, &mut place), false));
            }
            let cwd = place.cwd.clone().unwrap_or_default();
            let start = place.start_text();
            let Some(repo) = repository(&cwd, &place.issues).filter(|_| !cwd.is_empty() && !start.is_empty()) else { continue };
            let revision = revision(&repo, start);
            if revision.is_empty() {
                continue;
            }
            let worktree = git(Path::new(&cwd), &["rev-parse", "--show-toplevel"]).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(&cwd));
            let context = Context { worktree, managed: managed_paths(&repo, start) };
            let mut verdict = Verdict::default();
            for (t, main) in &transcripts {
                verdict.check(t, &context, *main);
            }
            let project = project_name(&cwd, Some(&repo));
            for rule in &verdict.applied {
                let entry = counts.entry((rule, revision.clone())).or_default();
                let found: Vec<&Finding> = verdict.findings.iter().filter(|f| f.rule == *rule).collect();
                let breaches = found.iter().filter(|f| !f.blocked).count() as u64;
                entry[0] += 1;
                entry[1] += u64::from(breaches > 0);
                entry[2] += breaches;
                entry[3] += found.len() as u64 - breaches;
            }
            for f in verdict.findings {
                let kind = if f.blocked { "blocked" } else { "breach" };
                rows.push(format!("{id}\t{project}\t{revision}\t{}\t{kind}\t{}\t{}", f.rule, f.location, f.detail));
            }
        }
    }
    if summary {
        println!("{SUMMARY_HEADER}");
        for ((rule, revision), [applied, breached, breaches, blocked]) in counts {
            println!("{rule}\t{revision}\t{applied}\t{breached}\t{breaches}\t{blocked}");
        }
    } else {
        println!("{HEADER}");
        for row in rows {
            println!("{row}");
        }
    }
}
