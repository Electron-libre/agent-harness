//! The managed entries of the project's settings (SYN-29 to SYN-36).

use serde_json::{json, Value};

use super::*;

const SETTINGS: &str = ".claude/settings.json";
const FRAGMENT: &str = "entries/.claude/settings.json";
const DENY: &str = "Bash(just waive-review)";
const RECORD: &str = "rust-script .claude/hooks/review.rs record";
const GATE: &str = "rust-script .claude/hooks/gate.rs";

fn hook(command: &str) -> Value {
    json!({"type": "command", "command": command})
}

/// A fragment that denies `DENY` and runs `RECORD` after `Skill` and `GATE` at `Stop`.
fn fragment() -> Value {
    json!({
        "permissions": {"deny": [DENY]},
        "hooks": {
            "PostToolUse": [{"matcher": "Skill", "hooks": [hook(RECORD)]}],
            "Stop": [{"hooks": [hook(GATE)]}],
        },
    })
}

/// The `setup` source with `fragment` as its managed entries.
fn with_fragment(fragment: &Value) -> (TempDir, TempDir) {
    let (source, root) = setup();
    put(source.path(), FRAGMENT, &fragment.to_string());
    (source, root)
}

fn settings(root: &TempDir) -> Value {
    serde_json::from_str(&get(root.path(), SETTINGS).unwrap()).unwrap()
}

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

// spec: SYN-29
#[test]
fn adds_each_managed_entry_the_settings_lack_and_keeps_the_rest() {
    let (_source, root) = with_fragment(&fragment());
    let mine = hook("my-formatter");
    put(
        root.path(),
        SETTINGS,
        &json!({
            "model": "opus",
            "permissions": {"allow": ["Bash(ls)"], "deny": ["Bash(rm *)"]},
            "hooks": {"PostToolUse": [
                {"matcher": "Edit", "hooks": [mine]},
                {"matcher": "Skill", "hooks": [hook("mine-too")]},
            ]},
        })
        .to_string(),
    );
    run(&root);
    let got = settings(&root);
    assert_eq!(got["model"], "opus");
    assert_eq!(got["permissions"]["allow"], json!(["Bash(ls)"]));
    assert_eq!(got["permissions"]["deny"], json!(["Bash(rm *)", DENY]));
    assert_eq!(
        got["hooks"]["PostToolUse"],
        json!([
            {"matcher": "Edit", "hooks": [hook("my-formatter")]},
            {"matcher": "Skill", "hooks": [hook("mine-too"), hook(RECORD)]},
        ])
    );
    assert_eq!(got["hooks"]["Stop"], json!([{"hooks": [hook(GATE)]}]));
}

// spec: SYN-29
#[test]
fn creates_the_keys_the_settings_lack() {
    let (_source, root) = with_fragment(&fragment());
    put(root.path(), SETTINGS, "{}");
    run(&root);
    assert_eq!(settings(&root), fragment());
}

// spec: SYN-29
#[test]
fn an_entry_the_settings_already_hold_is_not_added_again() {
    let (_source, root) = with_fragment(&fragment());
    let mut held = fragment();
    held["hooks"]["Stop"][0]["hooks"]
        .as_array_mut()
        .unwrap()
        .insert(0, hook("mine"));
    // key order differs from the fragment: JSON equality ignores it
    held["hooks"]["PostToolUse"][0]["hooks"][0] = json!({"command": RECORD, "type": "command"});
    let text = serde_json::to_string(&held).unwrap();
    put(root.path(), SETTINGS, &text);
    let report = run(&root);
    assert_eq!(get(root.path(), SETTINGS).unwrap(), text, "not written");
    assert!(
        report
            .entries
            .iter()
            .all(|(path, _)| !path.starts_with(SETTINGS)),
        "{:?}",
        report.entries
    );
}

// spec: SYN-30
#[test]
fn the_lock_records_each_managed_entry() {
    let (_source, root) = with_fragment(&fragment());
    put(root.path(), SETTINGS, "{}");
    run(&root);
    let lock = get(root.path(), LOCK).unwrap();
    let entries: Vec<&str> = lock.lines().filter(|l| l.starts_with("entry  ")).collect();
    assert_eq!(entries.len(), 3, "{lock}");
    // the lock still reads back, and its entries are no file
    assert!(check_with(root.path(), false).unwrap().problems.is_empty());
}

// spec: SYN-32
#[test]
fn keeps_the_key_order_and_writes_indented_json() {
    let (_source, root) = with_fragment(&fragment());
    put(
        root.path(),
        SETTINGS,
        r#"{"zeta": 1, "hooks": {"Stop": []}, "alpha": {"b": 1, "a": 2}}"#,
    );
    run(&root);
    let text = get(root.path(), SETTINGS).unwrap();
    let got: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(keys(&got), ["zeta", "hooks", "alpha", "permissions"]);
    assert_eq!(keys(&got["alpha"]), ["b", "a"]);
    assert_eq!(keys(&got["hooks"]), ["Stop", "PostToolUse"]);
    assert_eq!(
        text,
        format!("{}\n", serde_json::to_string_pretty(&got).unwrap())
    );
}

// spec: SYN-32
#[test]
fn a_second_sync_leaves_the_settings_byte_identical() {
    let (_source, root) = with_fragment(&fragment());
    put(root.path(), SETTINGS, "{}");
    run(&root);
    let first = get(root.path(), SETTINGS);
    let lock = get(root.path(), LOCK);
    let report = run(&root);
    assert_eq!(get(root.path(), SETTINGS), first);
    assert_eq!(get(root.path(), LOCK), lock);
    assert!(report
        .entries
        .iter()
        .all(|(_, outcome)| *outcome == Outcome::Unchanged));
}

// spec: SYN-34
#[test]
fn reports_each_added_entry_and_writes_nothing_under_dry_run() {
    let (_source, root) = with_fragment(&fragment());
    put(root.path(), SETTINGS, "{}");
    let report = sync(
        root.path(),
        &SyncOptions {
            force: false,
            dry_run: true,
        },
    )
    .unwrap();
    assert_eq!(get(root.path(), SETTINGS).as_deref(), Some("{}"));
    assert!(get(root.path(), LOCK).is_none());
    let added: Vec<&str> = report
        .entries
        .iter()
        .filter(|(path, _)| path.starts_with(SETTINGS))
        .map(|(path, outcome)| {
            assert_eq!(*outcome, Outcome::Added);
            path.as_str()
        })
        .collect();
    assert_eq!(
        added,
        [
            format!("{SETTINGS} deny {DENY}"),
            format!("{SETTINGS} hook PostToolUse Skill {RECORD}"),
            format!("{SETTINGS} hook Stop {GATE}"),
        ]
    );
}

// spec: SYN-34
#[test]
fn a_dry_run_after_seeding_reports_the_entries_of_the_seeded_settings() {
    let (source, root) = with_fragment(&fragment());
    put(
        source.path(),
        &format!("seed/{SETTINGS}"),
        r#"{"permissions": {"deny": ["Bash(just waive-review)"]}}"#,
    );
    let report = sync(
        root.path(),
        &SyncOptions {
            force: false,
            dry_run: true,
        },
    )
    .unwrap();
    assert_eq!(report.outcome(SETTINGS), Some(Outcome::Seeded));
    assert_eq!(report.outcome(&format!("{SETTINGS} deny {DENY}")), None);
    assert_eq!(
        report.outcome(&format!("{SETTINGS} hook Stop {GATE}")),
        Some(Outcome::Added)
    );
}

// spec: SYN-35
#[test]
fn settings_that_cannot_be_merged_are_left_alone_and_their_entries_kept() {
    let (_source, root) = with_fragment(&fragment());
    put(root.path(), SETTINGS, "{}");
    run(&root);
    let lock = get(root.path(), LOCK).unwrap();
    for text in [
        "{ not json",
        "[]",
        r#"{"hooks": []}"#,
        r#"{"permissions": {"deny": "all"}}"#,
    ] {
        put(root.path(), SETTINGS, text);
        let report = run(&root);
        assert_eq!(get(root.path(), SETTINGS).as_deref(), Some(text));
        assert_eq!(
            get(root.path(), LOCK).as_deref(),
            Some(lock.as_str()),
            "{text}"
        );
        assert_eq!(report.conflicts(), 0, "{text}");
        assert!(!report.warnings.is_empty(), "{text}: a warning says why");
    }
    fs::remove_file(root.path().join(SETTINGS)).unwrap();
    let report = run(&root);
    assert!(get(root.path(), SETTINGS).is_none());
    assert_eq!(get(root.path(), LOCK).as_deref(), Some(lock.as_str()));
    assert_eq!(report.conflicts(), 0);
}

// spec: SYN-29
#[test]
fn a_source_whose_fragment_is_not_valid_fails_and_writes_nothing() {
    let (source, root) = setup();
    put(
        source.path(),
        FRAGMENT,
        r#"{"hooks": {"Stop": [{"hooks": "x"}]}}"#,
    );
    put(root.path(), SETTINGS, "{}");
    assert!(sync(root.path(), &SyncOptions::default()).is_err());
    assert!(get(root.path(), "a.txt").is_none());
    assert!(get(root.path(), LOCK).is_none());
}

// spec: SYN-25, SYN-26, SYN-27, SYN-29
#[test]
fn the_repository_settings_once_synced_give_check_nothing_to_warn_of() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = tempdir().unwrap();
    let report = sync_from(root.path(), &repo, &SyncOptions::default()).unwrap();
    assert_eq!(report.warnings, Vec::<String>::new());
    let checked = check_with(root.path(), true).unwrap();
    assert_eq!(checked.warnings, Vec::<String>::new());
    assert_eq!(checked.problems, Vec::<String>::new());
}
