//! The native review gate installed by `sync` (SYN-18 to SYN-22).

use std::process::{Command, Output};

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

/// A directory holding a fake `harness` with `run` that records its arguments in `calls` and exits with `code`.
fn fake_harness(code: i32) -> TempDir {
    fake_with("harness", code, "[ \"$1\" = help ] && exit 0\n")
}

/// A directory holding a fake `program` that records its arguments in `calls` and exits with `code`.
fn fake(program: &str, code: i32) -> TempDir {
    fake_with(program, code, "")
}

/// `fake`, running `prelude` first.
fn fake_with(program: &str, code: i32, prelude: &str) -> TempDir {
    let bin = tempdir().unwrap();
    let script = bin.path().join(program);
    fs::write(
        &script,
        format!(
            "#!/bin/sh\n{prelude}echo \"$@\" >> \"{}/calls\"\nexit {code}\n",
            bin.path().display()
        ),
    )
    .unwrap();
    set_executable(&script, true).unwrap();
    bin
}

/// Run the installed hook in `root` with `path` as the whole PATH of the shell it starts.
fn run_hook(root: &TempDir, path: &Path, args: &[&str]) -> Output {
    Command::new("/bin/sh")
        .arg(".git/hooks/pre-commit")
        .args(args)
        .current_dir(root.path())
        .env("PATH", path)
        .output()
        .unwrap()
}

// spec: SYN-18
#[test]
fn installs_an_executable_hook_that_runs_the_review_in_native_mode() {
    let (_source, root) = gated();
    let report = run(&root);
    let text = get(root.path(), ".git/hooks/pre-commit").unwrap();
    assert!(
        text.starts_with("#!/bin/sh\n# harness: native review gate"),
        "{text}"
    );
    assert!(text.contains("harness run review-gate native"), "{text}");
    assert!(is_executable(&pre_commit(&root)));
    assert_eq!(report.outcome(gate::GATE), Some(Outcome::Added));
}

// spec: SYN-18
#[test]
fn the_hook_runs_the_review_from_the_work_tree_and_fails_with_its_exit_code() {
    let (_source, root) = gated();
    run(&root);
    for code in [0, 1, 2] {
        let bin = fake_harness(code);
        let out = run_hook(&root, bin.path(), &[]);
        assert_eq!(out.status.code(), Some(code));
        let calls = fs::read_to_string(bin.path().join("calls")).unwrap();
        assert_eq!(calls.trim(), "run review-gate native");
    }
}

// spec: SYN-18, BIN-14
#[test]
fn the_hook_lets_the_commit_through_when_the_launcher_cannot_run_the_review() {
    let (_source, root) = gated();
    run(&root);
    let bin = fake_harness(3);
    let out = run_hook(&root, bin.path(), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("review gate skipped"));
}

/// The review hook script of the revisions before the tools moved into `harness`.
const LEGACY_HOOK: &str = ".claude/hooks/require_code_review.rs";

// spec: SYN-18
#[test]
fn the_hook_runs_the_review_script_of_an_older_revision() {
    let (_source, root) = gated();
    run(&root);
    put(
        root.path(),
        LEGACY_HOOK,
        "// the review hook of an older revision",
    );
    let bin = fake("rust-script", 2);
    let out = run_hook(&root, bin.path(), &[]);
    assert_eq!(out.status.code(), Some(2));
    let calls = fs::read_to_string(bin.path().join("calls")).unwrap();
    assert_eq!(calls.trim(), format!("{LEGACY_HOOK} native"));
}

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

// spec: SYN-18, BIN-9
#[test]
fn the_hook_warns_and_lets_the_commit_through_without_the_harness() {
    let (_source, root) = gated();
    run(&root);
    let bin = fake_harness(3);
    // no harness.toml in the work tree (a branch that predates the harness)
    let config = get(root.path(), CONFIG).unwrap();
    fs::remove_file(root.path().join(CONFIG)).unwrap();
    let out = run_hook(&root, bin.path(), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("review gate skipped"));
    assert!(!bin.path().join("calls").exists());
    // harness.toml present, no harness on the PATH
    put(root.path(), CONFIG, &config);
    let empty = tempdir().unwrap();
    let out = run_hook(&root, empty.path(), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("review gate skipped"));
    // a harness that predates `run`: its `help run` fails, as any unknown subcommand
    let old = fake("harness", 2);
    let out = run_hook(&root, old.path(), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("review gate skipped"));
    let calls = fs::read_to_string(old.path().join("calls")).unwrap();
    assert_eq!(calls.trim(), "help run");
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

// spec: SYN-19
#[test]
fn moves_a_hook_of_the_user_aside_and_runs_it_after_the_gate() {
    let (_source, root) = gated();
    user_hook(&root, 0);
    let original = get(root.path(), ".git/hooks/pre-commit").unwrap();
    let report = run(&root);
    assert_eq!(
        get(root.path(), ".git/hooks/pre-commit.user").as_deref(),
        Some(original.as_str())
    );
    assert!(is_executable(
        &root.path().join(".git/hooks/pre-commit.user")
    ));
    assert_eq!(
        report.outcome("pre-commit -> pre-commit.user"),
        Some(Outcome::Moved)
    );

    let bin = fake_harness(0);
    let out = run_hook(&root, bin.path(), &["msg-file", "message"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        get(root.path(), "ran").as_deref(),
        Some("user msg-file message\n")
    );
}

// spec: SYN-19
#[test]
fn a_refusal_of_the_gate_stops_the_commit_before_the_hook_of_the_user() {
    let (_source, root) = gated();
    user_hook(&root, 0);
    run(&root);
    let bin = fake_harness(2);
    let out = run_hook(&root, bin.path(), &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(get(root.path(), "ran").is_none());
}

// spec: SYN-19
#[test]
fn a_failure_of_the_hook_of_the_user_fails_the_commit() {
    let (_source, root) = gated();
    user_hook(&root, 7);
    run(&root);
    let bin = fake_harness(0);
    assert_eq!(run_hook(&root, bin.path(), &[]).status.code(), Some(7));
}

// spec: SYN-19
#[test]
fn never_replaces_a_chained_hook_and_fails_changing_nothing() {
    let (_source, root) = gated();
    user_hook(&root, 0);
    fs::write(root.path().join(".git/hooks/pre-commit.user"), "kept").unwrap();
    let original = get(root.path(), ".git/hooks/pre-commit").unwrap();
    let err = sync(root.path(), &SyncOptions::default()).unwrap_err();
    assert!(format!("{err:#}").contains("pre-commit.user"), "{err:#}");
    assert_eq!(
        get(root.path(), ".git/hooks/pre-commit").as_deref(),
        Some(original.as_str())
    );
    assert_eq!(
        get(root.path(), ".git/hooks/pre-commit.user").as_deref(),
        Some("kept")
    );
    assert!(get(root.path(), "a.txt").is_none());
    assert!(get(root.path(), LOCK).is_none());
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

// spec: SYN-19
#[test]
fn a_hook_that_only_quotes_the_mark_is_the_users_and_a_symlinked_one_keeps_its_target() {
    let (_source, root) = gated();
    let hooks = root.path().join(".git/hooks");
    fs::create_dir_all(&hooks).unwrap();
    fs::write(
        pre_commit(&root),
        "#!/bin/sh\n# copied from: # harness: native review gate\nexit 0\n",
    )
    .unwrap();
    set_executable(&pre_commit(&root), true).unwrap();
    assert_eq!(
        run(&root).outcome("pre-commit -> pre-commit.user"),
        Some(Outcome::Moved)
    );
    assert!(get(root.path(), ".git/hooks/pre-commit.user")
        .unwrap()
        .contains("copied from"));

    // a relative link still reaches its script once the hook moved
    fs::remove_file(root.path().join(".git/hooks/pre-commit.user")).unwrap();
    fs::remove_file(pre_commit(&root)).unwrap();
    put(root.path(), "scripts/hook", "#!/bin/sh\nexit 0\n");
    set_executable(&root.path().join("scripts/hook"), true).unwrap();
    std::os::unix::fs::symlink("../../scripts/hook", pre_commit(&root)).unwrap();
    run(&root);
    let chained = root.path().join(".git/hooks/pre-commit.user");
    assert_eq!(fs::read_to_string(&chained).unwrap(), "#!/bin/sh\nexit 0\n");
    assert!(fs::symlink_metadata(pre_commit(&root)).unwrap().is_file());
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
