//! The hook commands the installed launcher must accept (spec `specs/tool-binaries.md`, BIN-15, BIN-16).
//!
//! The installed `cardinal` (or `harness`, its name before the rename) is the launcher every hook goes through: a hook
//! command `cardinal <subcommand> ...` it does not know does not run (it exits 3, BIN-17; a launcher older than BIN-17
//! exits 2, which Claude Code reads as a refusal, so the agent is locked out), and one whose launcher is not installed
//! runs nothing. `check` asks the launcher on the path, `<launcher> help <subcommand>`, before such settings take
//! effect, and `update` refuses a revision whose hooks it refuses (BIN-16).

use std::collections::BTreeMap;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::is_git_url;
use crate::run::cargo_url;
use crate::settings::hook_commands;

/// The launcher, then its name before the rename.
const LAUNCHERS: [&str; 2] = ["cardinal", "harness"];

/// Whether `launcher` accepts `sub` (`<launcher> help <sub>` succeeds); `None` when it cannot be run (not on the path).
fn accepts(launcher: &str, sub: &str) -> Option<bool> {
    Command::new(launcher)
        .args(["help", sub])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .map(|status| status.success())
}

/// The hook commands of `settings` whose first word is a launcher (`cardinal`, `harness`) and whose subcommand the
/// launcher of that name on the path refuses, in the order of the settings. A `cardinal` that cannot be run refuses
/// every subcommand when `harness` can (a machine not moved to the new name yet); a launcher that cannot be run, and
/// `harness` when neither can, refuse nothing.
pub fn refused(settings: &Value) -> Vec<String> {
    let unquoted = |word: &str| word.trim_matches(|c| matches!(c, '"' | '\'')).to_string();
    let mut known: BTreeMap<(String, String), Option<bool>> = BTreeMap::new();
    let mut refused = Vec::new();
    for command in hook_commands(settings) {
        let mut words = command.split_whitespace().map(unquoted);
        let (Some(first), Some(sub)) = (words.next(), words.next()) else {
            continue;
        };
        if !LAUNCHERS.contains(&first.as_str()) || sub.starts_with('-') {
            continue;
        }
        let mut ask = |launcher: &str, sub: &str| {
            *known
                .entry((launcher.to_string(), sub.to_string()))
                .or_insert_with(|| accepts(launcher, sub))
        };
        let accepted = match ask(&first, &sub) {
            None if first == LAUNCHERS[0] => ask(LAUNCHERS[1], "help").map(|_| false),
            accepted => accepted,
        };
        if accepted == Some(false) {
            refused.push(command.to_string());
        }
    }
    refused
}

/// The command that installs the `cardinal` of `rev` of `source`, run from the root of the project; `--force` replaces
/// the `harness` binary of a package installed before the rename.
pub fn install(source: &str, rev: Option<&str>) -> String {
    if !is_git_url(source) {
        return format!("cargo install --force --path {source}/crates/cardinal --locked");
    }
    let rev = rev.map(|rev| format!(" --rev {rev}")).unwrap_or_default();
    format!(
        "cargo install --force --git {}{rev} --locked",
        cargo_url(source)
    )
}
