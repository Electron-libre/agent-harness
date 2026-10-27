//! The settings warnings of `check`, `sync` and `update` (SYN-24 to SYN-27, REV-19).

use super::*;

/// The managed hook of the review gate.
const RECORD: &str = "harness run review-gate record";
/// Its report line once in the settings (SYN-34).
const RECORD_ENTRY: &str =
    ".claude/settings.json hook PostToolUse Skill harness run review-gate record";

/// Make `source` manage the hook of the review gate, and seed empty settings for it.
fn manage_the_review_gate(source: &Path) {
    let hook = serde_json::json!({"type": "command", "command": RECORD});
    let fragment =
        serde_json::json!({"hooks": {"PostToolUse": [{"matcher": "Skill", "hooks": [hook]}]}});
    put(
        source,
        "entries/.claude/settings.json",
        &fragment.to_string(),
    );
    put(source, "seed/.claude/settings.json", "{}");
}

/// A source that manages the review gate, and a git project pointing at it.
fn gated() -> (TempDir, TempDir) {
    let (source, root) = setup();
    manage_the_review_gate(source.path());
    git_in(root.path(), &["init", "--quiet", "-b", "main"]);
    (source, root)
}

// spec: REV-19
#[test]
fn the_managed_settings_entries_deny_the_waiver_to_the_agent() {
    let fragment =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../entries/.claude/settings.json");
    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fragment).unwrap()).unwrap();
    let deny: Vec<&str> = settings["permissions"]["deny"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r.as_str())
        .collect();
    for rule in settings::DENY_RULES {
        assert!(
            deny.contains(&rule),
            "the seeded settings lack the deny rule {rule}: {deny:?}"
        );
    }
}

// spec: SYN-24
#[test]
fn check_warns_of_each_deny_rule_the_settings_lack() {
    let (_source, root) = gated();
    run(&root);
    let settings = ".claude/settings.json";
    let warning = |rule: &str| format!("warning: {settings} lacks the deny rule {rule}");
    let deny_warnings = |root: &Path| -> Vec<String> {
        check_with(root, false, &no_harness())
            .unwrap()
            .warnings
            .into_iter()
            .filter(|w| w.contains("lacks the deny rule"))
            .collect()
    };
    // no settings file: nothing to warn about
    fs::remove_file(root.path().join(settings)).unwrap();
    assert!(check_with(root.path(), false, &no_harness())
        .unwrap()
        .warnings
        .is_empty());

    put(
        root.path(),
        settings,
        r#"{"permissions": {"allow": ["Bash(just waive-review)"]}}"#,
    );
    assert_eq!(
        deny_warnings(root.path()),
        settings::DENY_RULES
            .iter()
            .map(|r| warning(r))
            .collect::<Vec<_>>()
    );

    put(
        root.path(),
        settings,
        r#"{"permissions": {"deny": ["Bash(just waive-review)", "Bash(just waive-review *)"]}}"#,
    );
    assert_eq!(
        deny_warnings(root.path()),
        vec![warning("Edit(//**/claude-review-waived)")]
    );
    // a warning does not make `check` fail: the only problem is the hook entry these settings lack
    assert_eq!(
        check_with(root.path(), false, &no_harness())
            .unwrap()
            .problems,
        vec![format!("missing: {RECORD_ENTRY}")]
    );

    let all = serde_json::json!({"permissions": {"deny": settings::DENY_RULES}});
    put(root.path(), settings, &all.to_string());
    assert!(deny_warnings(root.path()).is_empty());

    // settings that do not parse are said so, not reported as lacking every rule
    put(root.path(), settings, "{,}");
    let invalid = check_with(root.path(), false, &no_harness())
        .unwrap()
        .warnings;
    assert!(
        invalid.len() == 1
            && invalid[0].starts_with(&format!("warning: {settings} is not valid JSON")),
        "{invalid:?}"
    );

    // a source that does not manage the review gate: no rule expected
    let (_source, plain) = setup();
    run(&plain);
    put(plain.path(), settings, "{}");
    assert!(deny_warnings(plain.path()).is_empty());
}

// spec: SYN-24, SYN-25, SYN-26
#[test]
fn unparsable_settings_get_one_warning_without_the_review_hook() {
    let (_source, plain) = setup();
    run(&plain);
    put(plain.path(), ".claude/settings.json", "{,}");
    let warnings = check_with(plain.path(), false, &no_harness())
        .unwrap()
        .warnings;
    assert!(
        warnings.len() == 1 && warnings[0].contains("is not valid JSON"),
        "{warnings:?}"
    );
}

/// `settings` with the attribution turned off, so that only the hooks are warned of.
fn quiet(hooks: &serde_json::Value) -> String {
    serde_json::json!({"attribution": {"commit": "", "pr": ""}, "hooks": hooks}).to_string()
}

// spec: SYN-25
#[test]
fn check_warns_of_each_hook_the_settings_run_and_the_project_lacks() {
    let (_source, root) = setup();
    run(&root);
    put(root.path(), ".claude/hooks/present.rs", "// here");
    let hooks = serde_json::json!({
        "PreToolUse": [{"matcher": "Bash", "hooks": [
            {"type": "command", "command": "rust-script \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/present.rs"},
            {"type": "command", "command": "rust-script \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/gone.rs check"},
        ]}],
        "Stop": [{"hooks": [
            {"type": "command", "command": "rust-script .claude/hooks/also_gone.rs", "timeout": 600},
            {"type": "command", "command": "just lint"},
            {"type": "command", "command": "bash ~/.claude/hooks/notify.sh; run-parts .claude/hooks/"},
            {"type": "command", "command": "sh -c \"$(cat .claude/hooks/present.rs)\" >/dev/null"},
            {"type": "command", "command": "rust-script ./.claude/hooks/gone.rs"},
        ]}],
    });
    put(root.path(), ".claude/settings.json", &quiet(&hooks));
    let checked = check_with(root.path(), false, &no_harness()).unwrap();
    assert_eq!(
        checked.warnings,
        vec![
            "warning: .claude/settings.json runs the missing hook .claude/hooks/gone.rs"
                .to_string(),
            "warning: .claude/settings.json runs the missing hook .claude/hooks/also_gone.rs"
                .to_string(),
        ]
    );
    assert!(checked.problems.is_empty());
}

// spec: SYN-26
#[test]
fn check_warns_when_the_settings_leave_the_attribution_on() {
    let (_source, root) = setup();
    run(&root);
    let warning = "warning: .claude/settings.json does not turn off the attribution".to_string();
    let warnings = |text: &str| {
        put(root.path(), ".claude/settings.json", text);
        check_with(root.path(), false, &no_harness())
            .unwrap()
            .warnings
    };
    assert_eq!(warnings("{}"), vec![warning.clone()]);
    assert_eq!(
        warnings(r#"{"attribution": {"commit": ""}}"#),
        vec![warning.clone()]
    );
    assert_eq!(
        warnings(r#"{"attribution": {"commit": "", "pr": "Made by a bot"}}"#),
        vec![warning.clone()]
    );
    assert_eq!(warnings(r#"{"attribution": true}"#), vec![warning]);
    assert!(warnings(r#"{"attribution": false}"#).is_empty());
    assert!(
        warnings(r#"{"attribution": {"commit": "", "pr": "", "sessionUrl": false}}"#).is_empty()
    );
    assert!(check_with(root.path(), false, &no_harness())
        .unwrap()
        .problems
        .is_empty());
}

const GONE: &str = "warning: .claude/settings.json runs the missing hook .claude/hooks/gone.rs";

/// Settings that run `.claude/hooks/gone.rs`, attribution off.
fn running_gone() -> String {
    quiet(&serde_json::json!({"Stop": [{"hooks": [
        {"type": "command", "command": "rust-script .claude/hooks/gone.rs"},
    ]}]}))
}

// spec: SYN-27
#[test]
fn sync_warns_of_the_hook_it_removed_that_the_settings_still_run() {
    let (source, root) = setup();
    put(source.path(), "files/.claude/hooks/gone.rs", "// a hook");
    put(root.path(), ".claude/settings.json", &running_gone());
    assert!(run(&root).warnings.is_empty());

    fs::remove_file(source.path().join("files/.claude/hooks/gone.rs")).unwrap();
    let report = run(&root);
    assert_eq!(
        report.outcome(".claude/hooks/gone.rs"),
        Some(Outcome::Removed)
    );
    assert_eq!(report.warnings, vec![GONE.to_string()]);
    // a warning is not a conflict: the sync succeeds
    assert_eq!(report.conflicts(), 0);
}

// spec: SYN-27
#[test]
fn update_reports_the_warnings_of_its_sync_once() {
    let (repo, _first, _second) = git_source();
    let root = git_project(&repo);
    put(root.path(), ".claude/settings.json", &running_gone());
    let cache = tempdir().unwrap();
    let done = update_in(
        Some(cache.path()),
        root.path(),
        None,
        &SyncOptions::default(),
        &no_harness(),
    )
    .unwrap();
    assert!(done.applied);
    assert_eq!(done.sync.warnings, vec![GONE.to_string()]);
}

// spec: SYN-27
#[test]
fn a_dry_run_warns_of_the_settings_as_the_sync_would_leave_them() {
    let dry = SyncOptions {
        force: false,
        dry_run: true,
    };
    let (source, root) = setup();
    put(source.path(), "files/.claude/hooks/gone.rs", "// a hook");
    put(root.path(), ".claude/settings.json", &running_gone());
    // the sync would add the hook the settings run: nothing to warn of, though it is not there yet
    let report = sync(root.path(), &dry).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(get(root.path(), ".claude/hooks/gone.rs").is_none());

    run(&root);
    fs::remove_file(source.path().join("files/.claude/hooks/gone.rs")).unwrap();
    let lock = get(root.path(), LOCK);
    // the sync would remove it: warned of, though it is still there
    let report = sync(root.path(), &dry).unwrap();
    assert_eq!(report.warnings, vec![GONE.to_string()]);
    assert!(get(root.path(), ".claude/hooks/gone.rs").is_some());
    assert_eq!(get(root.path(), LOCK), lock);
}
