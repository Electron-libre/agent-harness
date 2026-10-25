//! The native review gate installed by `sync` (SYN-18 to SYN-20).

use std::process::{Command, Output};

use super::*;

const HOOK: &str = ".claude/hooks/require_code_review.rs";

/// A source that ships the review hook, and a git project pointing at it.
fn gated() -> (TempDir, TempDir) {
    let (source, root) = setup();
    put(
        source.path(),
        &format!("files/{HOOK}"),
        "// the review hook",
    );
    git_in(root.path(), &["init", "--quiet", "-b", "main"]);
    (source, root)
}

fn pre_commit(root: &TempDir) -> PathBuf {
    root.path().join(".git/hooks/pre-commit")
}

/// A directory holding a fake `rust-script` that records its arguments in `calls` and exits with `code`.
fn fake_rust_script(code: i32) -> TempDir {
    let bin = tempdir().unwrap();
    let script = bin.path().join("rust-script");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\necho \"$@\" >> \"{}/calls\"\nexit {code}\n",
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
    assert!(text.contains(&format!("hook={HOOK}\n")), "{text}");
    assert!(text.contains("rust-script \"$hook\" native"), "{text}");
    assert!(is_executable(&pre_commit(&root)));
    assert_eq!(report.outcome(gate::GATE), Some(Outcome::Added));
}

// spec: SYN-18
#[test]
fn the_hook_runs_the_review_from_the_work_tree_and_fails_with_its_exit_code() {
    let (_source, root) = gated();
    run(&root);
    put(root.path(), HOOK, "// the review hook");
    for code in [0, 3] {
        let bin = fake_rust_script(code);
        let out = run_hook(&root, bin.path(), &[]);
        assert_eq!(out.status.code(), Some(code));
        let calls = fs::read_to_string(bin.path().join("calls")).unwrap();
        assert_eq!(calls.trim(), format!("{HOOK} native"));
    }
}

// spec: SYN-18
#[test]
fn the_hook_warns_and_lets_the_commit_through_without_the_review_hook_or_rust_script() {
    let (_source, root) = gated();
    run(&root);
    let bin = fake_rust_script(3);
    // no review hook in the work tree (a branch that predates the harness)
    fs::remove_file(root.path().join(HOOK)).unwrap();
    let out = run_hook(&root, bin.path(), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("review gate skipped"));
    assert!(!bin.path().join("calls").exists());
    // review hook present, no rust-script on the PATH
    put(root.path(), HOOK, "// the review hook");
    let empty = tempdir().unwrap();
    let out = run_hook(&root, empty.path(), &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("review gate skipped"));
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
    put(root.path(), HOOK, "// the review hook");

    let bin = fake_rust_script(0);
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
    put(root.path(), HOOK, "// the review hook");
    let bin = fake_rust_script(2);
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
    put(root.path(), HOOK, "// the review hook");
    let bin = fake_rust_script(0);
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
