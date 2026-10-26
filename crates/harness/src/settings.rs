//! The warnings of `check` on the project's `.claude/settings.json` (`specs/sync.md`, SYN-24 to SYN-26). Seeded once,
//! the file is never rewritten by `sync`, so a project learns this way of what a later harness expects of it.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde_json::Value;

/// The `permissions.deny` rules that keep the review waiver from the agent (REV-19).
pub const DENY_RULES: [&str; 3] = [
    "Bash(just waive-review)",
    "Bash(just waive-review *)",
    "Edit(//**/claude-review-waived)",
];
/// The settings of the project, relative to its root.
const SETTINGS: &str = ".claude/settings.json";
/// Where the hooks the settings run live, relative to the root of the project.
const HOOKS: &str = ".claude/hooks/";
/// What may come before `HOOKS` in a word of a hook command, quotes removed, for the path to be the project's.
const PROJECT_PREFIXES: [&str; 4] = ["", "./", "$CLAUDE_PROJECT_DIR/", "${CLAUDE_PROJECT_DIR}/"];

/// The warnings on the project's settings, none when it has none: the deny rules they lack when `gated` (the project
/// has the review hook), the hooks they run that the project lacks, and an attribution left on. One warning saying
/// why when they cannot be read or parsed.
pub fn warnings(root: &Path, gated: bool) -> Vec<String> {
    let settings = match fs::read_to_string(root.join(SETTINGS)) {
        Err(e) if e.kind() == ErrorKind::NotFound => return Vec::new(),
        Err(e) => return vec![format!("warning: {SETTINGS} cannot be read: {e}")],
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Err(e) => return vec![format!("warning: {SETTINGS} is not valid JSON: {e}")],
            Ok(settings) => settings,
        },
    };
    let mut warnings = if gated {
        missing_deny_rules(&settings)
    } else {
        Vec::new()
    };
    warnings.extend(missing_hooks(root, &settings));
    if !attribution_off(&settings) {
        warnings.push(format!(
            "warning: {SETTINGS} does not turn off the attribution"
        ));
    }
    warnings
}

/// One warning per rule of `DENY_RULES` missing from `permissions.deny` (SYN-24).
fn missing_deny_rules(settings: &Value) -> Vec<String> {
    let deny: Vec<&str> = settings["permissions"]["deny"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    DENY_RULES
        .iter()
        .filter(|rule| !deny.contains(rule))
        .map(|rule| format!("warning: {SETTINGS} lacks the deny rule {rule}"))
        .collect()
}

/// One warning per file under the project's `HOOKS` that a hook command names and the project lacks, in the order of
/// the settings (SYN-25). A path under another directory, a directory or a glob is not looked at.
fn missing_hooks(root: &Path, settings: &Value) -> Vec<String> {
    let commands = settings["hooks"]
        .as_object()
        .into_iter()
        .flat_map(|events| events.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|matcher| matcher["hooks"].as_array())
        .flatten()
        .filter_map(|hook| hook["command"].as_str());
    let mut missing: Vec<String> = Vec::new();
    for word in commands.flat_map(str::split_whitespace) {
        let word: String = word.chars().filter(|c| !matches!(c, '"' | '\'')).collect();
        let Some(at) = word.find(HOOKS) else { continue };
        if !PROJECT_PREFIXES.contains(&&word[..at]) {
            continue;
        }
        let name: String = word[at + HOOKS.len()..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
            .collect();
        let path = format!("{HOOKS}{name}");
        if !name.is_empty() && !root.join(&path).is_file() && !missing.contains(&path) {
            missing.push(path);
        }
    }
    missing
        .iter()
        .map(|path| format!("warning: {SETTINGS} runs the missing hook {path}"))
        .collect()
}

/// Whether `attribution` is `false`, or holds an empty `commit` and an empty `pr` (SYN-26).
fn attribution_off(settings: &Value) -> bool {
    let attribution = &settings["attribution"];
    attribution == &Value::Bool(false)
        || ["commit", "pr"]
            .iter()
            .all(|key| attribution[*key].as_str() == Some(""))
}
