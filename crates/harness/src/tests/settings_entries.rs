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
    assert!(check_with(root.path(), false, &no_harness())
        .unwrap()
        .problems
        .is_empty());
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
    let checked = check_with(root.path(), true, &no_harness()).unwrap();
    assert_eq!(checked.warnings, Vec::<String>::new());
    assert_eq!(checked.problems, Vec::<String>::new());
}

/// The `with_fragment` project, synced once from settings holding a hook of its own at `Stop`.
fn synced() -> (TempDir, TempDir) {
    let (source, root) = with_fragment(&fragment());
    put(
        root.path(),
        SETTINGS,
        &json!({"hooks": {"Stop": [{"hooks": [hook("mine")]}]}}).to_string(),
    );
    run(&root);
    (source, root)
}

fn forced(root: &TempDir) -> Report {
    sync(
        root.path(),
        &SyncOptions {
            force: true,
            dry_run: false,
        },
    )
    .unwrap()
}

// spec: SYN-31
#[test]
fn removes_the_entries_the_source_dropped_and_the_groups_they_empty() {
    let (source, root) = synced();
    put(source.path(), FRAGMENT, "{}");
    let report = run(&root);
    assert_eq!(
        settings(&root),
        json!({"hooks": {"Stop": [{"hooks": [hook("mine")]}]}, "permissions": {"deny": []}})
    );
    assert_eq!(
        report.outcome(&format!("{SETTINGS} hook PostToolUse Skill {RECORD}")),
        Some(Outcome::Removed)
    );
    assert_eq!(
        report.outcome(&format!("{SETTINGS} deny {DENY}")),
        Some(Outcome::Removed)
    );
    assert!(!get(root.path(), LOCK).unwrap().contains("entry  "));
    assert_eq!(report.conflicts(), 0);
}

// spec: SYN-31
#[test]
fn a_changed_hook_replaces_the_old_one_in_its_group() {
    let (source, root) = synced();
    let mut changed = fragment();
    changed["hooks"]["Stop"][0]["hooks"][0] = hook("gate");
    put(source.path(), FRAGMENT, &changed.to_string());
    let report = run(&root);
    assert_eq!(
        settings(&root)["hooks"]["Stop"],
        json!([{"hooks": [hook("mine"), hook("gate")]}])
    );
    assert_eq!(
        report.outcome(&format!("{SETTINGS} hook Stop {GATE}")),
        Some(Outcome::Removed)
    );
    assert_eq!(
        report.outcome(&format!("{SETTINGS} hook Stop gate")),
        Some(Outcome::Added)
    );
    // replaced once: the next sync has nothing to do
    let text = get(root.path(), SETTINGS);
    run(&root);
    assert_eq!(get(root.path(), SETTINGS), text);
}

// spec: SYN-31
#[test]
fn a_changed_hook_alone_in_its_group_keeps_its_place() {
    let (source, root) = with_fragment(&fragment());
    let before = json!({"permissions": {"deny": [DENY]}, "hooks": {
        "PostToolUse": [
            {"matcher": "Skill", "hooks": [hook(RECORD)]},
            {"matcher": "Edit", "hooks": [hook("mine")]},
        ],
        "Stop": [{"hooks": [hook(GATE)]}],
        "PreCompact": [{"hooks": [hook("mine")]}],
    }});
    put(root.path(), SETTINGS, &before.to_string());
    run(&root);
    let mut changed = fragment();
    changed["hooks"]["PostToolUse"][0]["hooks"][0] = hook("record");
    changed["hooks"]["Stop"][0]["hooks"][0] = hook("gate");
    put(source.path(), FRAGMENT, &changed.to_string());
    run(&root);
    let mut expected = before;
    expected["hooks"]["PostToolUse"][0]["hooks"][0] = hook("record");
    expected["hooks"]["Stop"][0]["hooks"][0] = hook("gate");
    let got = settings(&root);
    assert_eq!(got, expected);
    assert_eq!(keys(&got["hooks"]), ["PostToolUse", "Stop", "PreCompact"]);
    assert_eq!(got["hooks"]["PostToolUse"][0]["matcher"], "Skill");
}

// spec: SYN-29
#[test]
fn an_empty_or_star_matcher_is_the_same_as_none() {
    let (_source, root) = with_fragment(&fragment());
    let held = json!({"hooks": {"Stop": [{"matcher": "", "hooks": [hook(GATE)]}],
        "PostToolUse": [{"matcher": "Skill", "hooks": [hook(RECORD)]}]},
        "permissions": {"deny": [DENY]}});
    put(root.path(), SETTINGS, &held.to_string());
    run(&root);
    assert_eq!(settings(&root), held);
    put(
        root.path(),
        SETTINGS,
        &held
            .to_string()
            .replace(r#""matcher":"""#, r#""matcher":"*""#),
    );
    run(&root);
    assert_eq!(
        settings(&root)["hooks"]["Stop"][0]["hooks"],
        json!([hook(GATE)])
    );
}

// spec: SYN-33
#[test]
fn settings_seeded_again_get_the_managed_entries_without_conflict() {
    let (source, root) = synced();
    put(source.path(), &format!("seed/{SETTINGS}"), "{}");
    fs::remove_file(root.path().join(SETTINGS)).unwrap();
    let report = run(&root);
    assert_eq!(report.conflicts(), 0, "{:?}", report.entries);
    assert_eq!(settings(&root), fragment());
}

// spec: SYN-36
#[test]
fn check_reports_nothing_of_settings_it_cannot_judge() {
    let (_source, root) = synced();
    for text in [None, Some("{ not json")] {
        match text {
            None => fs::remove_file(root.path().join(SETTINGS)).unwrap(),
            Some(text) => put(root.path(), SETTINGS, text),
        }
        assert!(
            check_with(root.path(), false, &no_harness())
                .unwrap()
                .problems
                .is_empty(),
            "{text:?}"
        );
    }
}

// spec: SYN-31
#[test]
fn a_dropped_entry_the_project_edited_is_left_as_it_is() {
    let (source, root) = synced();
    let mut edited = settings(&root);
    edited["hooks"]["Stop"][0]["hooks"][1]["timeout"] = json!(5);
    put(root.path(), SETTINGS, &edited.to_string());
    let mut dropped = fragment();
    dropped["hooks"].as_object_mut().unwrap().remove("Stop");
    put(source.path(), FRAGMENT, &dropped.to_string());
    let report = run(&root);
    assert_eq!(settings(&root)["hooks"]["Stop"], edited["hooks"]["Stop"]);
    assert_eq!(report.conflicts(), 0);
    // said once, as for a dropped file modified locally
    assert_eq!(
        report.outcome(&format!("{SETTINGS} hook Stop {GATE}")),
        Some(Outcome::Kept)
    );
    assert_eq!(
        run(&root).outcome(&format!("{SETTINGS} hook Stop {GATE}")),
        None
    );
}

// spec: SYN-33
#[test]
fn a_managed_entry_the_project_removed_or_edited_is_a_conflict_unless_forced() {
    let (_source, root) = synced();
    let mut edited = settings(&root);
    edited["permissions"]["deny"] = json!([]);
    edited["hooks"]["Stop"][0]["hooks"][1]["timeout"] = json!(5);
    put(root.path(), SETTINGS, &edited.to_string());
    let text = get(root.path(), SETTINGS);
    let lock = get(root.path(), LOCK);

    let report = run(&root);
    assert_eq!(report.conflicts(), 2, "{:?}", report.entries);
    assert_eq!(
        report.outcome(&format!("{SETTINGS} deny {DENY}")),
        Some(Outcome::Conflict)
    );
    assert_eq!(get(root.path(), SETTINGS), text, "not added back");
    assert_eq!(get(root.path(), LOCK), lock, "still recorded");
    // the conflict survives the next run
    assert_eq!(run(&root).conflicts(), 2);

    let report = forced(&root);
    assert_eq!(report.conflicts(), 0);
    let got = settings(&root);
    assert_eq!(got["permissions"]["deny"], json!([DENY]));
    assert_eq!(
        got["hooks"]["Stop"][0]["hooks"],
        json!([
            hook("mine"),
            edited["hooks"]["Stop"][0]["hooks"][1],
            hook(GATE)
        ])
    );
}

// spec: SYN-36
#[test]
fn check_reports_each_recorded_entry_the_settings_no_longer_hold() {
    let (_source, root) = synced();
    assert!(check_with(root.path(), false, &no_harness())
        .unwrap()
        .problems
        .is_empty());
    let mut edited = settings(&root);
    edited["permissions"]["deny"] = json!(["Bash(rm *)"]);
    edited["hooks"]["Stop"][0]["hooks"][0] = hook("not mine any more");
    put(root.path(), SETTINGS, &edited.to_string());
    assert_eq!(
        check_with(root.path(), false, &no_harness())
            .unwrap()
            .problems,
        vec![format!("missing: {SETTINGS} deny {DENY}")]
    );
}
