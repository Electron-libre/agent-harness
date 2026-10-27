//! The native review gate `sync` installs, run as the binary on temp dirs, and the `pre-commit` it writes, run with
//! `/bin/sh` (SYN-18, SYN-19, BIN-9, BIN-14).
#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{executable, get, output, put, script, snapshot, Home, Run, CONFIG};
use tempfile::{tempdir, TempDir};

const GATE: &str = "native review gate";
const MOVED: &str = "pre-commit -> pre-commit.user";
const SKIPPED: &str = "review gate skipped";

/// A source managing the hook of the review gate, and a git project initialised on it, not synced yet.
fn gated(home: &Home) -> (TempDir, TempDir) {
    let source = tempdir().unwrap();
    put(source.path(), "files/a.txt", "a1");
    let hook = serde_json::json!({"type": "command", "command": "harness run review-gate record"});
    let fragment =
        serde_json::json!({"hooks": {"PostToolUse": [{"matcher": "Skill", "hooks": [hook]}]}});
    put(
        source.path(),
        "entries/.claude/settings.json",
        &fragment.to_string(),
    );
    put(source.path(), "seed/.claude/settings.json", "{}");
    let root = tempdir().unwrap();
    home.git(root.path(), &["init", "--quiet", "-b", "main"]);
    home.ok(root.path(), &["init", source.path().to_str().unwrap()]);
    (source, root)
}

/// `gated`, synced.
fn synced(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = gated(home);
    home.ok(root.path(), &["sync"]);
    (source, root)
}

fn pre_commit(root: &TempDir) -> PathBuf {
    root.path().join(".git/hooks/pre-commit")
}

/// A directory holding only a fake `program` that records its arguments in `calls`, then runs `body`.
fn fake(program: &str, body: &str) -> TempDir {
    let bin = tempdir().unwrap();
    script(
        &bin.path().join(program),
        &format!("echo \"$@\" >> '{}/calls'\n{body}", bin.path().display()),
    );
    bin
}

/// A `harness` that knows `run` and whose `run` exits with `code`.
fn fake_harness(code: i32) -> TempDir {
    fake(
        "harness",
        &format!("[ \"$1\" = help ] && exit 0\nexit {code}\n"),
    )
}

/// What the fake of `bin` was called with, one line per call; empty when it never ran.
fn calls(bin: &TempDir) -> String {
    get(bin.path(), "calls").unwrap_or_default()
}

/// Run the installed hook in `root`, as git does, with `path` as the whole `PATH` of its shell.
fn run_hook(home: &Home, root: &TempDir, path: &Path, args: &[&str]) -> Run {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg(".git/hooks/pre-commit")
        .args(args)
        .current_dir(root.path())
        .env("PATH", path);
    home.isolated(&mut cmd);
    output(cmd, "")
}

/// A `pre-commit` of the user that records its arguments in `ran` and exits with `code`.
fn user_hook(root: &TempDir, code: i32) {
    script(
        &pre_commit(root),
        &format!("echo \"user $@\" >> ran\nexit {code}\n"),
    );
}

// spec: SYN-18
#[test]
fn sync_installs_an_executable_hook_that_runs_the_review_in_native_mode() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    assert!(!pre_commit(&root).exists(), "init alone installs no gate");
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(GATE), Some("added"), "{sync:?}");
    let text = get(root.path(), ".git/hooks/pre-commit").unwrap();
    assert!(
        text.starts_with("#!/bin/sh\n# harness: native review gate"),
        "{text}"
    );
    assert!(text.contains("harness run review-gate native"), "{text}");
    assert!(executable(&pre_commit(&root)));
}

// spec: SYN-18
#[test]
fn the_hook_runs_the_review_and_fails_the_commit_with_its_exit_code() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    for code in [0, 1, 2] {
        let bin = fake_harness(code);
        let hook = run_hook(&home, &root, bin.path(), &[]);
        assert_eq!(hook.code, Some(code), "{hook:?}");
        assert_eq!(calls(&bin), "help run\nrun review-gate native\n");
    }
}

// spec: SYN-18, BIN-14
#[test]
fn the_hook_lets_the_commit_through_when_the_launcher_cannot_run_the_review() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    let bin = fake_harness(3);
    let hook = run_hook(&home, &root, bin.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
}

/// The review hook script of the revisions before the tools moved into `harness`.
const LEGACY_HOOK: &str = ".claude/hooks/require_code_review.rs";

// spec: SYN-18
#[test]
fn the_hook_runs_the_review_script_of_an_older_revision_with_rust_script() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    put(
        root.path(),
        LEGACY_HOOK,
        "// the review hook of an older revision",
    );
    let bin = fake("rust-script", "exit 2\n");
    let hook = run_hook(&home, &root, bin.path(), &[]);
    assert_eq!(hook.code, Some(2), "{hook:?}");
    assert_eq!(calls(&bin), format!("{LEGACY_HOOK} native\n"));
    // its script without `rust-script`: skipped, not refused
    let empty = tempdir().unwrap();
    let hook = run_hook(&home, &root, empty.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
}

// spec: SYN-18, BIN-9
#[test]
fn the_hook_warns_and_lets_the_commit_through_without_the_harness() {
    let home = Home::new();
    let (_source, root) = synced(&home);
    // no harness.toml in the work tree (a branch that predates the harness)
    let config = get(root.path(), CONFIG).unwrap();
    fs::remove_file(root.path().join(CONFIG)).unwrap();
    let bin = fake_harness(2);
    let hook = run_hook(&home, &root, bin.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
    assert_eq!(calls(&bin), "");
    // harness.toml present, no harness on the PATH
    put(root.path(), CONFIG, &config);
    let empty = tempdir().unwrap();
    let hook = run_hook(&home, &root, empty.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
    // a harness that predates `run`: its `help run` fails, as any unknown subcommand
    let old = fake("harness", "exit 2\n");
    let hook = run_hook(&home, &root, old.path(), &[]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert!(hook.stderr.contains(SKIPPED), "{hook:?}");
    assert_eq!(calls(&old), "help run\n");
}

// spec: SYN-19
#[test]
fn sync_moves_a_hook_of_the_user_aside_and_the_gate_runs_it_after_the_review() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    user_hook(&root, 0);
    let original = get(root.path(), ".git/hooks/pre-commit").unwrap();
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(MOVED), Some("moved"), "{sync:?}");
    assert_eq!(sync.outcome(GATE), Some("added"), "{sync:?}");
    let chained = root.path().join(".git/hooks/pre-commit.user");
    assert_eq!(fs::read_to_string(&chained).unwrap(), original);
    assert!(executable(&chained));

    let bin = fake_harness(0);
    let hook = run_hook(&home, &root, bin.path(), &["msg-file", "message"]);
    assert_eq!(hook.code, Some(0), "{hook:?}");
    assert_eq!(calls(&bin), "help run\nrun review-gate native\n");
    assert_eq!(
        get(root.path(), "ran").as_deref(),
        Some("user msg-file message\n")
    );
}

// spec: SYN-19
#[test]
fn a_refusal_of_the_gate_stops_the_commit_before_the_hook_of_the_user() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    user_hook(&root, 0);
    home.ok(root.path(), &["sync"]);
    let bin = fake_harness(2);
    assert_eq!(run_hook(&home, &root, bin.path(), &[]).code, Some(2));
    assert!(get(root.path(), "ran").is_none());
}

// spec: SYN-19
#[test]
fn a_failure_of_the_hook_of_the_user_fails_the_commit() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    user_hook(&root, 7);
    home.ok(root.path(), &["sync"]);
    let bin = fake_harness(0);
    assert_eq!(run_hook(&home, &root, bin.path(), &[]).code, Some(7));
    assert_eq!(get(root.path(), "ran").as_deref(), Some("user \n"));
}

// spec: SYN-19
#[test]
fn sync_never_replaces_a_chained_hook_and_fails_changing_nothing() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    user_hook(&root, 0);
    put(root.path(), ".git/hooks/pre-commit.user", "kept");
    let before = snapshot(root.path());
    let sync = home.harness(root.path(), &["sync"]);
    assert_ne!(sync.code, Some(0), "{sync:?}");
    assert!(sync.stderr.contains("pre-commit.user"), "{sync:?}");
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-19
#[test]
fn a_hook_that_only_quotes_the_mark_is_the_users() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    script(
        &pre_commit(&root),
        "# copied from: # harness: native review gate\nexit 0\n",
    );
    let sync = home.ok(root.path(), &["sync"]);
    assert_eq!(sync.outcome(MOVED), Some("moved"), "{sync:?}");
    assert!(get(root.path(), ".git/hooks/pre-commit.user")
        .unwrap()
        .contains("copied from"));
}

// spec: SYN-19
#[test]
fn a_symlinked_hook_of_the_user_still_reaches_its_target_once_moved() {
    let home = Home::new();
    let (_source, root) = gated(&home);
    script(&root.path().join("scripts/hook"), "echo linked >> ran\n");
    fs::create_dir_all(root.path().join(".git/hooks")).unwrap();
    std::os::unix::fs::symlink("../../scripts/hook", pre_commit(&root)).unwrap();
    home.ok(root.path(), &["sync"]);
    assert!(fs::symlink_metadata(pre_commit(&root)).unwrap().is_file());
    let bin = fake_harness(0);
    assert_eq!(run_hook(&home, &root, bin.path(), &[]).code, Some(0));
    assert_eq!(get(root.path(), "ran").as_deref(), Some("linked\n"));
}
