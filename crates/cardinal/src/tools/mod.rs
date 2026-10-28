//! The tools of the harness, run by `cardinal tool <name>` (and so by `cardinal run <name>`, which picks the binary
//! of the pinned revision). Each reads its arguments, standard input and current directory, and returns its exit code.

use std::path::{Path, PathBuf};
use std::process::Command;

mod audit_sources;
mod cardinal_changes;
mod cardinal_report;
mod check_commits;
mod check_issue;
mod check_specs;
mod clippy_gate;
mod close_issue;
mod commit_check;
mod delivered_quality;
mod fixed_context;
mod format_rust;
mod issue_worktree;
mod language;
mod language_hint;
mod next_issue;
mod repo_files;
mod review_gate;
mod rule_breaches;
mod session_metrics;
mod spec_coverage;
mod transcripts;

pub(crate) use fixed_context::description_bytes;

/// A tool: its name on the command line, a summary, and its entry point.
pub struct Tool {
    pub name: &'static str,
    pub about: &'static str,
    pub main: fn(&[String]) -> u8,
}

/// Every tool, by name.
pub const TOOLS: &[Tool] = &[
    Tool {
        name: "audit-sources",
        about: "[projects_dir] [--messages]: the instruction sources and typed messages of every project",
        main: audit_sources::main,
    },
    Tool {
        name: "cardinal-changes",
        about: "[repo] --sessions <file>: the expected effects of the harness changes, with the revisions holding each",
        main: cardinal_changes::main,
    },
    Tool {
        name: "cardinal-report",
        about: "--sessions <file> [...]: the markdown report of the readers, by group of revisions",
        main: cardinal_report::main,
    },
    Tool {
        name: "check-commits",
        about: "validate the message of every non-merge commit of <base>..<head>",
        main: check_commits::main,
    },
    Tool {
        name: "check-issue",
        about: "validate a new issue file against the create-issue template",
        main: check_issue::main,
    },
    Tool {
        name: "check-specs",
        about: "[--strict] [root]: validate specs/*.md and trace their requirements to the tests",
        main: check_specs::main,
    },
    Tool {
        name: "clippy-gate",
        about: "Stop hook: block the end of a turn while clippy reports a problem",
        main: clippy_gate::main,
    },
    Tool {
        name: "close-issue",
        about: "move a finished issue to issues/done/ and repoint the documents that cite it",
        main: close_issue::main,
    },
    Tool {
        name: "commit-check",
        about: "validate a commit message read from stdin against AGENTS.md",
        main: commit_check::main,
    },
    Tool {
        name: "delivered-quality",
        about: "[repo] [--summary]: one row per done issue, its fixes, reviews and failed checks",
        main: delivered_quality::main,
    },
    Tool {
        name: "fixed-context",
        about: "[projects_dir]: the bytes every session pays, per project and harness revision",
        main: fixed_context::main,
    },
    Tool {
        name: "format-rust",
        about: "PostToolUse hook: cargo fmt the project of an edited .rs file",
        main: format_rust::main,
    },
    Tool {
        name: "issue-worktree",
        about: "start <issue> | finish <stem> [--force]: the worktree and branch of an issue",
        main: issue_worktree::main,
    },
    Tool {
        name: "language-hint",
        about: "Stop hook: send the agent back once when its last reply is not in the user_language",
        main: language_hint::main,
    },
    Tool {
        name: "next-issue",
        about: "the next issue of issues/queue.md no branch or worktree has taken",
        main: next_issue::main,
    },
    Tool {
        name: "review-gate",
        about: "record | check | native: the code review gate of a commit",
        main: review_gate::main,
    },
    Tool {
        name: "rule-breaches",
        about: "[projects_dir] [--summary]: where the agent broke a rule a program can check",
        main: rule_breaches::main,
    },
    Tool {
        name: "session-metrics",
        about: "[projects_dir] [--archive <file>]: one row of numbers per session of every project",
        main: session_metrics::main,
    },
    Tool {
        name: "spec-coverage",
        about: "[root]: the test files that cite no requirement of an accepted spec, largest first",
        main: spec_coverage::main,
    },
];

/// Run the tool `args[0]` with the rest; exit 3 with the list of the tools when there is none or it is unknown (BIN-7).
#[must_use]
pub fn run(args: &[String]) -> u8 {
    let found = args
        .first()
        .and_then(|name| TOOLS.iter().find(|t| t.name == name));
    if let Some(tool) = found {
        return (tool.main)(&args[1..]);
    }
    match args.first() {
        Some(name) => eprintln!("cardinal: unknown tool `{name}`; the tools are:"),
        None => eprintln!("cardinal: name a tool; the tools are:"),
    }
    for tool in TOOLS {
        eprintln!("  {:<16} {}", tool.name, tool.about);
    }
    crate::NOT_A_REFUSAL
}

/// The fenced code block open after `line`, given the one `open` before it (its character and length): a line
/// starting, indent allowed, with three or more backticks (none after them, else it is inline code) or tildes opens
/// one; a line of only its character, at least as many, closes it.
fn fence(open: Option<(char, usize)>, line: &str) -> Option<(char, usize)> {
    let Some((c, n)) = open else {
        let line = line.trim_start();
        let c = line.chars().next().filter(|c| *c == '`' || *c == '~')?;
        let rest = line.trim_start_matches(c);
        let n = line.len() - rest.len();
        return (n >= 3 && !(c == '`' && rest.contains('`'))).then_some((c, n));
    };
    let line = line.trim();
    open.filter(|_| line.len() < n || !line.chars().all(|x| x == c))
}

/// Whether each line of `lines` belongs to a fenced code block, its fence lines included.
fn fenced<'a>(lines: impl IntoIterator<Item = &'a str>) -> impl Iterator<Item = (&'a str, bool)> {
    let mut open = None;
    lines.into_iter().map(move |line| {
        let before = open;
        open = fence(open, line);
        (line, before.is_some() || open.is_some())
    })
}

/// Root of the repository holding the current directory.
fn repo_root() -> Result<PathBuf, String> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("cannot run git to find the repository root: {e}"))?;
    if !out.status.success() {
        return Err("not inside a git repository: cannot locate .claude/cardinal.toml".into());
    }
    let root = String::from_utf8_lossy(&out.stdout);
    Ok(root.strip_suffix('\n').unwrap_or(&root).into())
}

/// Strings of `key = ["a", "b"]` in the `[project]` table of the configuration (`crate::config_of`); empty when the key is absent.
fn config_list(root: &Path, key: &str) -> Result<Vec<String>, String> {
    let path = crate::config_of(root);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("invalid {}: {e}", path.display()))?;
    let list = table
        .get("project")
        .and_then(|p| p.get(key))
        .or_else(|| table.get(key));
    Ok(list
        .and_then(toml::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default())
}
