//! The native review gate installed by `sync` and the settings warnings (SYN-20 to SYN-27, REV-19).

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

fn pre_commit(root: &TempDir) -> PathBuf {
    root.path().join(".git/hooks/pre-commit")
}

/// The review hook script of the revisions before the tools moved into `harness`.
const LEGACY_HOOK: &str = ".claude/hooks/require_code_review.rs";

// spec: SYN-22, SYN-23
#[test]
fn a_source_of_an_older_revision_still_gets_and_checks_the_gate() {
    let (source, root) = setup();
    put(
        source.path(),
        &format!("files/{LEGACY_HOOK}"),
        "// the review hook",
    );
    git_in(root.path(), &["init", "--quiet", "-b", "main"]);
    let report = run(&root);
    assert_eq!(report.outcome(gate::GATE), Some(Outcome::Added));
    fs::remove_file(pre_commit(&root)).unwrap();
    assert_eq!(
        check_with(root.path(), false, &no_harness())
            .unwrap()
            .problems,
        vec!["missing: native review gate"]
    );
}

/// A user's `pre-commit` that records its arguments in `ran` and exits with `code`.
fn user_hook(root: &TempDir, code: i32) {
    let path = pre_commit(root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        format!("#!/bin/sh\necho \"user $@\" >> ran\nexit {code}\n"),
    )
    .unwrap();
    set_executable(&path, true).unwrap();
}

// spec: SYN-20
#[test]
fn reinstalls_a_missing_modified_or_non_executable_hook_and_is_idempotent() {
    let (_source, root) = gated();
    run(&root);
    let managed = get(root.path(), ".git/hooks/pre-commit").unwrap();
    assert_eq!(run(&root).outcome(gate::GATE), Some(Outcome::Unchanged));

    fs::remove_file(pre_commit(&root)).unwrap();
    assert_eq!(run(&root).outcome(gate::GATE), Some(Outcome::Added));

    fs::write(pre_commit(&root), format!("{managed}# edited\n")).unwrap();
    set_executable(&pre_commit(&root), true).unwrap();
    assert_eq!(run(&root).outcome(gate::GATE), Some(Outcome::Updated));
    assert_eq!(
        get(root.path(), ".git/hooks/pre-commit").as_deref(),
        Some(managed.as_str())
    );

    set_executable(&pre_commit(&root), false).unwrap();
    assert_eq!(run(&root).outcome(gate::GATE), Some(Outcome::Updated));
    assert!(is_executable(&pre_commit(&root)));

    assert_eq!(run(&root).outcome(gate::GATE), Some(Outcome::Unchanged));
    assert!(get(root.path(), ".git/hooks/pre-commit.user").is_none());
}

// spec: SYN-20
#[test]
fn a_dry_run_reports_the_installation_and_writes_nothing() {
    let (_source, root) = gated();
    let report = sync(
        root.path(),
        &SyncOptions {
            force: false,
            dry_run: true,
        },
    )
    .unwrap();
    assert_eq!(report.outcome(gate::GATE), Some(Outcome::Added));
    assert!(!pre_commit(&root).exists());
    assert!(get(root.path(), LOCK).is_none());

    user_hook(&root, 0);
    let report = sync(
        root.path(),
        &SyncOptions {
            force: false,
            dry_run: true,
        },
    )
    .unwrap();
    assert_eq!(
        report.outcome("pre-commit -> pre-commit.user"),
        Some(Outcome::Moved)
    );
    assert!(!root.path().join(".git/hooks/pre-commit.user").exists());
}

/// The source of `gated()` serves a project at `dir`, which gets the configuration of `root`.
fn point_at_source(root: &TempDir, dir: &Path) {
    put(dir, CONFIG, &get(root.path(), CONFIG).unwrap());
}

// spec: SYN-21
#[test]
fn a_linked_worktree_gets_the_gate_in_the_hooks_of_the_common_git_directory() {
    let (_source, root) = gated();
    put(root.path(), "f.txt", "x");
    git_in(root.path(), &["add", "f.txt"]);
    git_in(root.path(), &["commit", "--quiet", "-m", "init"]);
    let linked = tempdir().unwrap();
    let linked_dir = linked.path().join("wt");
    git_in(
        root.path(),
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "other",
            &linked_dir.to_string_lossy(),
        ],
    );
    point_at_source(&root, &linked_dir);

    let report = sync(&linked_dir, &SyncOptions::default()).unwrap();
    assert_eq!(report.outcome(gate::GATE), Some(Outcome::Added));
    assert!(
        get(root.path(), ".git/hooks/pre-commit").is_some_and(|t| t.contains("native review gate"))
    );
    let private = fs::read_dir(root.path().join(".git/worktrees"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(!private.join("hooks").exists());
    // once for all the worktrees
    assert_eq!(
        sync(&linked_dir, &SyncOptions::default())
            .unwrap()
            .outcome(gate::GATE),
        Some(Outcome::Unchanged)
    );
}

// spec: SYN-22
#[test]
fn installs_nothing_when_core_hooks_path_is_set_and_says_why() {
    let (_source, root) = gated();
    let elsewhere = tempdir().unwrap();
    git_in(
        root.path(),
        &[
            "config",
            "core.hooksPath",
            &elsewhere.path().to_string_lossy(),
        ],
    );
    let report = sync(root.path(), &SyncOptions::default()).unwrap();
    let outcome = report.outcome(gate::GATE).unwrap();
    assert!(matches!(outcome, Outcome::Skipped(_)), "{outcome:?}");
    assert!(outcome.to_string().contains("core.hooksPath"), "{outcome}");
    assert!(!pre_commit(&root).exists());
    assert!(fs::read_dir(elsewhere.path()).unwrap().next().is_none());
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));

    // a relative path, the usual versioned directory, counts too
    let (_source, root) = gated();
    git_in(root.path(), &["config", "core.hooksPath", ".githooks"]);
    let report = sync(root.path(), &SyncOptions::default()).unwrap();
    assert!(matches!(
        report.outcome(gate::GATE),
        Some(Outcome::Skipped(_))
    ));
    assert!(!root.path().join(".githooks").exists());
    assert!(!pre_commit(&root).exists());
}

// spec: SYN-22
#[test]
fn no_gate_and_no_error_outside_a_git_repository_or_without_the_review_hook() {
    let (source, root) = setup();
    manage_the_review_gate(source.path());
    let report = run(&root);
    assert!(report.outcome(gate::GATE).is_none());
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));

    let (_source, root) = setup();
    git_in(root.path(), &["init", "--quiet", "-b", "main"]);
    let report = run(&root);
    assert!(report.outcome(gate::GATE).is_none());
    assert!(!pre_commit(&root).exists());

    // a project inside another repository is not the top of one
    let (source, root) = setup();
    manage_the_review_gate(source.path());
    let outer = tempdir().unwrap();
    git_in(outer.path(), &["init", "--quiet", "-b", "main"]);
    let inner = outer.path().join("project");
    point_at_source(&root, &inner);
    let report = sync(&inner, &SyncOptions::default()).unwrap();
    assert!(report.outcome(gate::GATE).is_none());
    assert!(!outer.path().join(".git/hooks/pre-commit").exists());
}

// spec: SYN-23
#[test]
fn check_reports_a_missing_or_modified_gate_and_nothing_else() {
    let (_source, root) = gated();
    run(&root);
    assert!(check_with(root.path(), false, &no_harness())
        .unwrap()
        .problems
        .is_empty());

    let managed = get(root.path(), ".git/hooks/pre-commit").unwrap();
    fs::write(pre_commit(&root), format!("{managed}# edited\n")).unwrap();
    set_executable(&pre_commit(&root), true).unwrap();
    assert_eq!(
        check_with(root.path(), false, &no_harness())
            .unwrap()
            .problems,
        vec!["modified: native review gate"]
    );

    fs::write(pre_commit(&root), managed.clone()).unwrap();
    set_executable(&pre_commit(&root), false).unwrap();
    assert_eq!(
        check_with(root.path(), false, &no_harness())
            .unwrap()
            .problems,
        vec!["modified: native review gate"]
    );

    fs::remove_file(pre_commit(&root)).unwrap();
    assert_eq!(
        check_with(root.path(), false, &no_harness())
            .unwrap()
            .problems,
        vec!["missing: native review gate"]
    );

    // a hook of the user without ours is a missing gate too
    user_hook(&root, 0);
    assert_eq!(
        check_with(root.path(), false, &no_harness())
            .unwrap()
            .problems,
        vec!["missing: native review gate"]
    );
    fs::remove_file(pre_commit(&root)).unwrap();

    // a gate that cannot be installed is not a problem to report
    git_in(root.path(), &["config", "core.hooksPath", ".githooks"]);
    assert!(check_with(root.path(), false, &no_harness())
        .unwrap()
        .problems
        .is_empty());
    git_in(root.path(), &["config", "--unset", "core.hooksPath"]);

    // the hook of the review gate removed from the settings since the sync is still recorded: the gate is still expected
    put(root.path(), ".claude/settings.json", "{}");
    assert_eq!(
        check_with(root.path(), false, &no_harness())
            .unwrap()
            .problems,
        vec![
            format!("missing: {RECORD_ENTRY}"),
            "missing: native review gate".to_string()
        ]
    );
    // a fresh clone of a CI run has no git hooks
    assert_eq!(
        check_with(root.path(), true, &no_harness())
            .unwrap()
            .problems,
        vec![format!("missing: {RECORD_ENTRY}")]
    );

    // a source that does not manage the review gate: no gate expected
    let (_source, plain) = setup();
    git_in(plain.path(), &["init", "--quiet", "-b", "main"]);
    run(&plain);
    assert!(check_with(plain.path(), false, &no_harness())
        .unwrap()
        .problems
        .is_empty());

    // not a repository
    let (source, root) = setup();
    manage_the_review_gate(source.path());
    run(&root);
    assert!(check_with(root.path(), false, &no_harness())
        .unwrap()
        .problems
        .is_empty());
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
