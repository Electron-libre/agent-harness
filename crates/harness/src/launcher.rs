//! The hook commands the installed `harness` must accept (spec `specs/tool-binaries.md`, BIN-15, BIN-16).
//!
//! The installed `harness` is the launcher every hook goes through: a hook command `harness <subcommand> ...` it does not
//! know exits 2, which Claude Code reads as a refusal, so the agent is locked out. `check` asks the `harness` on the
//! path, `harness help <subcommand>`, before such settings take effect, and `update` refuses a revision whose hooks it
//! refuses (BIN-16).

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::is_git_url;
use crate::run::cargo_url;
use crate::settings::hook_commands;

/// The hook commands of `settings` whose first word is `harness` and whose subcommand `harness` refuses, in the order
/// of the settings; none when `harness` cannot be run (not on the path), and no more once a run of it fails.
pub fn refused(settings: &Value, harness: &OsStr) -> Vec<String> {
    let unquoted = |word: &str| word.trim_matches(|c| matches!(c, '"' | '\'')).to_string();
    let mut known: BTreeMap<String, bool> = BTreeMap::new();
    let mut refused = Vec::new();
    for command in hook_commands(settings) {
        let mut words = command.split_whitespace().map(unquoted);
        let (Some(first), Some(sub)) = (words.next(), words.next()) else {
            continue;
        };
        if first != "harness" || sub.starts_with('-') {
            continue;
        }
        let accepted = match known.get(&sub) {
            Some(&accepted) => accepted,
            None => {
                let Ok(status) = Command::new(harness)
                    .args(["help", &sub])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                else {
                    break;
                };
                *known.entry(sub).or_insert(status.success())
            }
        };
        if !accepted {
            refused.push(command.to_string());
        }
    }
    refused
}

/// The command that installs the `harness` of `rev` of `source`, run from the root of the project.
pub fn install(source: &str, rev: Option<&str>) -> String {
    if !is_git_url(source) {
        return format!("cargo install --path {source}/crates/harness --locked");
    }
    let rev = rev.map(|rev| format!(" --rev {rev}")).unwrap_or_default();
    format!("cargo install --git {}{rev} --locked", cargo_url(source))
}
