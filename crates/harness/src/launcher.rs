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

use crate::settings::hook_commands;

/// The hook commands of `settings` whose first word is `harness` and whose subcommand `harness` refuses, in the order
/// of the settings; none when `harness` cannot be run (not on the path).
pub fn refused(settings: &Value, harness: &OsStr) -> Vec<String> {
    let mut known: BTreeMap<&str, bool> = BTreeMap::new();
    let mut refused = Vec::new();
    for command in hook_commands(settings) {
        let mut words = command.split_whitespace();
        let (Some("harness"), Some(sub)) = (words.next(), words.next()) else {
            continue;
        };
        if sub.starts_with('-') {
            continue;
        }
        let accepted = match known.get(sub) {
            Some(&accepted) => accepted,
            None => {
                let Ok(status) = Command::new(harness)
                    .args(["help", sub])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                else {
                    return Vec::new();
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
