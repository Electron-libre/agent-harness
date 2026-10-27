//! The tools of the harness, run by `harness tool <name>` (and so by `harness run <name>`, which picks the binary
//! of the pinned revision). Each reads its arguments, standard input and current directory, and returns its exit code.

use std::path::{Path, PathBuf};
use std::process::Command;

mod check_commits;
mod clippy_gate;
mod commit_check;
mod format_rust;
mod review_gate;

/// A tool: its name on the command line, a summary, and its entry point.
pub struct Tool {
    pub name: &'static str,
    pub about: &'static str,
    pub main: fn(&[String]) -> u8,
}

/// Every tool, by name.
pub const TOOLS: &[Tool] = &[
    Tool {
        name: "check-commits",
        about: "validate the message of every non-merge commit of <base>..<head>",
        main: check_commits::main,
    },
    Tool {
        name: "clippy-gate",
        about: "Stop hook: block the end of a turn while clippy reports a problem",
        main: clippy_gate::main,
    },
    Tool {
        name: "commit-check",
        about: "validate a commit message read from stdin against AGENTS.md",
        main: commit_check::main,
    },
    Tool {
        name: "format-rust",
        about: "PostToolUse hook: cargo fmt the project of an edited .rs file",
        main: format_rust::main,
    },
    Tool {
        name: "review-gate",
        about: "record | check | native: the code review gate of a commit",
        main: review_gate::main,
    },
];

/// Run the tool `args[0]` with the rest; exit 2 with the list of the tools when there is none or it is unknown (BIN-7).
#[must_use]
pub fn run(args: &[String]) -> u8 {
    let found = args
        .first()
        .and_then(|name| TOOLS.iter().find(|t| t.name == name));
    if let Some(tool) = found {
        return (tool.main)(&args[1..]);
    }
    match args.first() {
        Some(name) => eprintln!("harness: unknown tool `{name}`; the tools are:"),
        None => eprintln!("harness: name a tool; the tools are:"),
    }
    for tool in TOOLS {
        eprintln!("  {:<16} {}", tool.name, tool.about);
    }
    2
}

/// Root of the repository holding the current directory.
fn repo_root() -> Result<PathBuf, String> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("cannot run git to find the repository root: {e}"))?;
    if !out.status.success() {
        return Err("not inside a git repository: cannot locate .claude/harness.toml".into());
    }
    let root = String::from_utf8_lossy(&out.stdout);
    Ok(root.strip_suffix('\n').unwrap_or(&root).into())
}

/// Strings of `key = ["a", "b"]` in the `[project]` table of `.claude/harness.toml`; empty when the key is absent.
fn config_list(root: &Path, key: &str) -> Result<Vec<String>, String> {
    let path = root.join(crate::CONFIG);
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
