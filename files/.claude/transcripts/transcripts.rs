//! Reading of the Claude Code transcripts, shared by the scripts that read them (`#[path]` include, ADR 0006):
//! the projects directory, its projects and sessions, the sub-agent transcripts of a session, the parsed lines, the
//! filter of the messages the user typed, and what a session is resolved to (its repository, project, start and the
//! harness revision then). `rust-script` rebuilds a script only when its own text changes, so each
//! script that includes this module carries its SHA-256 (`// transcripts.rs sha256: <hex>`, checked by the tests).
#![allow(dead_code)]

use chrono::{DateTime, FixedOffset};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Branches that name no issue.
pub const DEFAULT_BRANCHES: &[&str] = &["main", "master", "HEAD"];

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

/// Calls `f` on each parsed line of a transcript, one at a time; returns the count of non-empty lines that are not
/// valid JSON.
pub fn for_each_line(path: &Path, mut f: impl FnMut(&Value)) -> std::io::Result<usize> {
    for_each_numbered_line(path, |_, value| f(value))
}

/// `for_each_line`, giving `f` the 1-based number of the line in the file too.
pub fn for_each_numbered_line(path: &Path, mut f: impl FnMut(usize, &Value)) -> std::io::Result<usize> {
    use std::io::BufRead;
    let mut unreadable = 0;
    for (n, line) in std::io::BufReader::new(std::fs::File::open(path)?).split(b'\n').enumerate() {
        let line = line?;
        if line.trim_ascii().is_empty() {
            continue;
        }
        match serde_json::from_slice::<Value>(&line) {
            Ok(value) => f(n + 1, &value),
            Err(_) => unreadable += 1,
        }
    }
    Ok(unreadable)
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

/// The text of a tool result: its string content, or its text parts joined.
pub fn result_text(block: &Value) -> String {
    match &block["content"] {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

fn push_distinct(list: &mut Vec<String>, value: Option<&str>) {
    if let Some(v) = value.filter(|v| !v.is_empty() && !list.iter().any(|k| k == v)) {
        list.push(v.to_string());
    }
}

/// What places a session: its first `cwd` (own transcript), the branches other than the default ones, and its
/// earliest and latest timestamps, sub-agents included.
#[derive(Default)]
pub struct Place {
    pub cwd: Option<String>,
    pub issues: Vec<String>,
    pub start: Option<(DateTime<FixedOffset>, String)>,
    pub end: Option<DateTime<FixedOffset>>,
}

impl Place {
    /// Takes in one line; returns false when it carries no valid `timestamp`.
    pub fn observe(&mut self, line: &Value, main: bool) -> bool {
        if main && self.cwd.is_none() {
            self.cwd = line["cwd"].as_str().map(str::to_string);
        }
        push_distinct(&mut self.issues, line["gitBranch"].as_str().filter(|b| !DEFAULT_BRANCHES.contains(b)));
        let Some((at, text)) = line["timestamp"].as_str().and_then(|t| DateTime::parse_from_rfc3339(t).ok().map(|d| (d, t))) else { return false };
        if self.start.as_ref().is_none_or(|(s, _)| at < *s) {
            self.start = Some((at, text.to_string()));
        }
        if self.end.is_none_or(|e| at > e) {
            self.end = Some(at);
        }
        true
    }

    /// The text of the earliest timestamp, empty when none.
    pub fn start_text(&self) -> &str {
        self.start.as_ref().map(|(_, text)| text.as_str()).unwrap_or_default()
    }
}

pub fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().ok().filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The main worktree of the repository holding `cwd`: a linked worktree counts with its main repository, a submodule
/// or a separate git directory is its own work tree. A removed issue worktree `<repo>-<branch>` (ADR 0003) counts
/// with `<repo>` when that one still exists.
pub fn repository(cwd: &str, branches: &[String]) -> Option<PathBuf> {
    let dir = Path::new(cwd);
    if !dir.exists() {
        let name = dir.file_name()?.to_string_lossy().into_owned();
        let repo = branches.iter().find_map(|b| name.strip_suffix(&format!("-{b}")).map(|r| dir.with_file_name(r)))?;
        return repository(repo.to_str()?, &[]);
    }
    let common = PathBuf::from(git(dir, &["rev-parse", "--path-format=absolute", "--git-common-dir"])?);
    if common.file_name().is_some_and(|n| n == ".git") {
        common.parent().map(Path::to_path_buf)
    } else {
        git(dir, &["rev-parse", "--show-toplevel"]).map(PathBuf::from)
    }
}

/// The project of a session: the name of its repository, else the last component of `cwd`.
pub fn project_name(cwd: &str, repo: Option<&Path>) -> String {
    repo.and_then(Path::file_name).or_else(|| Path::new(cwd).file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn default_branch(repo: &Path) -> Option<&'static str> {
    ["origin/HEAD", "main", "master"].into_iter().find(|b| git(repo, &["rev-parse", "--verify", "-q", &format!("{b}^{{commit}}")]).is_some())
}

/// The last commit of the first-parent history of the default branch of `repo` touching `path` at or before `at`.
fn last_commit(repo: &Path, at: &str, path: &str) -> Option<String> {
    let branch = default_branch(repo)?;
    git(repo, &["log", "-1", "--first-parent", "--format=%H", &format!("--before={at}"), branch, "--", path]).filter(|c| !c.is_empty())
}

/// The text of `path` as committed on the default branch of `repo` at `at`; `None` when it cannot be read.
pub fn committed_file(repo: &Path, at: &str, path: &str) -> Option<String> {
    let commit = last_commit(repo, at, path)?;
    git(repo, &["show", &format!("{commit}:{path}")])
}

/// The `.claude/harness.toml` committed on the default branch of `repo` at `at`, parsed.
pub fn committed_config(repo: &Path, at: &str) -> Option<toml::Table> {
    committed_file(repo, at, ".claude/harness.toml")?.parse().ok()
}

/// The harness revision in force in `repo` at `start` (ADR 0002): the `rev` of the `.claude/harness.toml` committed
/// on the default branch then; `self:<commit>` for the harness repository itself (`source = "."`, no `rev`), the
/// last commit touching `files/`; `unpinned` with no `rev`; empty when it cannot be read.
pub fn revision(repo: &Path, start: &str) -> String {
    let Some(config) = committed_config(repo, start) else { return String::new() };
    let harness = config.get("harness");
    let field = |key: &str| harness.and_then(|h| h.get(key)).and_then(toml::Value::as_str);
    match (field("rev"), field("source")) {
        (Some(rev), _) => rev.to_string(),
        (None, Some(".")) => last_commit(repo, start, "files").map(|c| format!("self:{c}")).unwrap_or_default(),
        (None, _) => "unpinned".to_string(),
    }
}
