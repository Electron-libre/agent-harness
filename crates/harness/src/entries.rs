//! The entries of the project's `.claude/settings.json` that the source manages (`specs/sync.md`, SYN-29 to SYN-36,
//! ADR 0007): the deny rules and hook objects of `entries/.claude/settings.json`, added to the settings by `sync` and
//! recorded in the lock, every other key left as the project wrote it.

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};

use crate::Outcome;

/// The settings of the project, relative to its root.
pub const SETTINGS: &str = ".claude/settings.json";
/// The managed entries, relative to the source.
const FRAGMENT: &str = "entries/.claude/settings.json";

/// One managed entry: a rule of `permissions.deny`, or a hook object of an event and matcher of `hooks`.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Deny(String),
    Hook {
        event: String,
        matcher: Option<String>,
        hook: Value,
    },
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::Deny(rule) => write!(f, "deny {rule}"),
            Entry::Hook {
                event,
                matcher,
                hook,
            } => {
                write!(f, "hook {event} ")?;
                if let Some(matcher) = matcher {
                    write!(f, "{matcher} ")?;
                }
                match hook["command"].as_str() {
                    Some(command) => f.write_str(command),
                    None => write!(f, "{hook}"),
                }
            }
        }
    }
}

impl Entry {
    /// The entry as one line of JSON, for the lock.
    pub fn to_lock(&self) -> String {
        match self {
            Entry::Deny(rule) => json!({ "deny": rule }),
            Entry::Hook {
                event,
                matcher,
                hook,
            } => json!({ "event": event, "matcher": matcher, "hook": hook }),
        }
        .to_string()
    }

    /// The entry of a line written by `to_lock`.
    pub fn from_lock(line: &str) -> Result<Entry> {
        let value: Value = serde_json::from_str(line)?;
        if let Some(rule) = value["deny"].as_str() {
            return Ok(Entry::Deny(rule.to_string()));
        }
        match (&value["event"], &value["matcher"], &value["hook"]) {
            (Value::String(event), Value::String(_) | Value::Null, hook @ Value::Object(_)) => {
                Ok(Entry::Hook {
                    event: event.clone(),
                    matcher: value["matcher"].as_str().map(String::from),
                    hook: hook.clone(),
                })
            }
            _ => bail!("not a settings entry: {line}"),
        }
    }

    /// The report line of the entry, `.claude/settings.json <entry>` (SYN-34).
    pub fn label(&self) -> String {
        format!("{SETTINGS} {self}")
    }
}

/// The entries the source manages, in the order of its fragment; none when it has no fragment. An error when the
/// fragment is not in the shape of SYN-29.
pub fn of_source(source: &Path) -> Result<Vec<Entry>> {
    let path = source.join(FRAGMENT);
    let text = match fs::read_to_string(&path) {
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        other => other.with_context(|| format!("reading {}", path.display()))?,
    };
    let fragment: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    entries(&fragment)
        .with_context(|| format!("{}: not in the shape of the settings", path.display()))
}

/// Every deny rule and hook of `settings`, or why its shape is not the one of SYN-29.
fn entries(settings: &Value) -> Result<Vec<Entry>> {
    let Some(settings) = settings.as_object() else {
        bail!("not an object");
    };
    let mut entries = Vec::new();
    if let Some(permissions) = settings.get("permissions") {
        let Some(permissions) = permissions.as_object() else {
            bail!("`permissions` is not an object");
        };
        if let Some(deny) = permissions.get("deny") {
            for rule in deny
                .as_array()
                .context("`permissions.deny` is not an array")?
            {
                let rule = rule.as_str().context("a deny rule is not a string")?;
                entries.push(Entry::Deny(rule.to_string()));
            }
        }
    }
    if let Some(hooks) = settings.get("hooks") {
        for (event, groups) in hooks.as_object().context("`hooks` is not an object")? {
            for group in groups
                .as_array()
                .with_context(|| format!("`hooks.{event}` is not an array"))?
            {
                let matcher = match &group["matcher"] {
                    Value::Null => None,
                    Value::String(matcher) => Some(matcher.clone()),
                    _ => bail!("a matcher of `hooks.{event}` is not a string"),
                };
                let list = group["hooks"]
                    .as_array()
                    .with_context(|| format!("a group of `hooks.{event}` has no `hooks` array"))?;
                for hook in list {
                    entries.push(Entry::Hook {
                        event: event.clone(),
                        matcher: matcher.clone(),
                        hook: hook.clone(),
                    });
                }
            }
        }
    }
    Ok(entries)
}

/// What the merge of the managed entries into the project's settings gives.
pub struct Merge {
    /// The settings as the sync leaves them, `None` when there are none it can merge into.
    pub settings: Option<Value>,
    /// The text to write, `None` when the file stays as it is.
    pub write: Option<String>,
    /// The entries for the lock.
    pub lock: Vec<Entry>,
    /// One line per entry added, labelled as SYN-34 says.
    pub outcomes: Vec<(String, Outcome)>,
    /// Why the settings were left alone, when they were not merged although they exist.
    pub warning: Option<String>,
}

/// The merge of `wanted` into the settings `text` (`None`: no settings), the lock having `recorded`.
pub fn merge(text: Option<&str>, wanted: &[Entry], recorded: &[Entry]) -> Merge {
    let left_alone = |settings, warning| Merge {
        settings,
        write: None,
        lock: recorded.to_vec(),
        outcomes: Vec::new(),
        warning,
    };
    let Some(text) = text else {
        return left_alone(None, None);
    };
    let Ok(mut settings) = serde_json::from_str::<Value>(text) else {
        // the warnings on the settings already say it is not valid JSON
        return left_alone(None, None);
    };
    if let Err(e) = entries(&settings) {
        let warning = format!(
            "warning: {SETTINGS} is not in the shape of Claude Code settings ({e}); its managed entries are not synced"
        );
        return left_alone(Some(settings), Some(warning));
    }
    let mut outcomes = Vec::new();
    for entry in wanted {
        if !holds(&settings, entry) {
            add(&mut settings, entry);
            outcomes.push((entry.label(), Outcome::Added));
        }
    }
    let write = (!outcomes.is_empty()).then(|| {
        let mut text = serde_json::to_string_pretty(&settings).expect("a JSON value serialises");
        text.push('\n');
        text
    });
    Merge {
        settings: Some(settings),
        write,
        lock: wanted.to_vec(),
        outcomes,
        warning: None,
    }
}

/// The groups of `event` in `settings` whose matcher is `matcher`.
fn groups<'a>(
    settings: &'a Value,
    event: &'a str,
    matcher: Option<&'a str>,
) -> impl Iterator<Item = &'a Value> {
    settings["hooks"][event]
        .as_array()
        .into_iter()
        .flatten()
        .filter(move |group| group["matcher"].as_str() == matcher)
}

/// Whether `settings` hold `entry` (JSON equality, key order ignored).
fn holds(settings: &Value, entry: &Entry) -> bool {
    match entry {
        Entry::Deny(rule) => settings["permissions"]["deny"]
            .as_array()
            .is_some_and(|deny| deny.iter().any(|r| r.as_str() == Some(rule))),
        Entry::Hook {
            event,
            matcher,
            hook,
        } => groups(settings, event, matcher.as_deref()).any(|group| {
            group["hooks"]
                .as_array()
                .is_some_and(|hooks| hooks.contains(hook))
        }),
    }
}

/// `settings` with `entry` appended (SYN-29); `settings` is in the shape of SYN-29.
fn add(settings: &mut Value, entry: &Entry) {
    let settings = settings.as_object_mut().expect("checked by `entries`");
    match entry {
        Entry::Deny(rule) => {
            let permissions = object(settings, "permissions");
            array(permissions, "deny").push(Value::String(rule.clone()));
        }
        Entry::Hook {
            event,
            matcher,
            hook,
        } => {
            let groups = array(object(settings, "hooks"), event);
            let found = groups
                .iter_mut()
                .find(|group| group["matcher"].as_str() == matcher.as_deref());
            match found {
                Some(group) => group["hooks"]
                    .as_array_mut()
                    .expect("checked by `entries`")
                    .push(hook.clone()),
                None => {
                    let mut group = Map::new();
                    if let Some(matcher) = matcher {
                        group.insert("matcher".into(), Value::String(matcher.clone()));
                    }
                    group.insert("hooks".into(), json!([hook]));
                    groups.push(Value::Object(group));
                }
            }
        }
    }
}

/// The object at `key` of `map`, created empty when missing.
fn object<'a>(map: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    map.entry(key)
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("checked by `entries`")
}

/// The array at `key` of `map`, created empty when missing.
fn array<'a>(map: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    map.entry(key)
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .expect("checked by `entries`")
}
