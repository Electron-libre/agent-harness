//! The hook commands the installed `harness` must accept (spec `specs/tool-binaries.md`), with a fake `harness` on no
//! network.
#![cfg(unix)]

use std::ffi::OsString;

use super::run::script;
use super::*;

/// A `harness` that knows only the subcommand `run`, logging each call to `log`.
fn harness_with_run(dir: &Path, log: &Path) -> OsString {
    script(
        &dir.join("harness"),
        &format!(
            "echo \"$*\" >> {log}\n[ \"$1\" = help ] && [ \"$2\" = run ]\n",
            log = log.display()
        ),
    )
}

const HOOKS: &str = r#"{"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [
    {"type": "command", "command": "harness run review-gate check"},
    {"type": "command", "command": "harness newcmd --flag"},
    {"type": "command", "command": "sh -c 'harness newcmd'"}
]}], "Stop": [{"hooks": [{"type": "command", "command": "harness newcmd again"}]}]}}"#;

/// A synced project whose settings hold `HOOKS`.
fn project_with_hooks() -> (TempDir, TempDir) {
    let (source, root) = setup();
    run(&root);
    put(root.path(), entries::SETTINGS, HOOKS);
    (source, root)
}

// spec: BIN-15
#[test]
fn check_fails_on_each_hook_command_whose_subcommand_the_installed_harness_refuses() {
    let (_source, root) = project_with_hooks();
    let bin = tempdir().unwrap();
    let log = bin.path().join("log");
    let harness = harness_with_run(bin.path(), &log);

    let problems = check_with(root.path(), false, &harness).unwrap().problems;
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(
        problems[0].contains("`harness newcmd --flag`"),
        "{problems:?}"
    );
    assert!(
        problems[1].contains("`harness newcmd again`"),
        "{problems:?}"
    );
    assert!(
        problems.iter().all(|p| p.contains("install")),
        "{problems:?}"
    );
    // each subcommand is asked once
    assert_eq!(fs::read_to_string(&log).unwrap(), "help run\nhelp newcmd\n");
}

// spec: BIN-15
#[test]
fn check_looks_at_nothing_without_a_harness_on_the_path() {
    let (_source, root) = project_with_hooks();
    let missing = root.path().join("no-harness").into_os_string();
    assert!(check_with(root.path(), false, &missing)
        .unwrap()
        .problems
        .is_empty());
}

/// A git source whose head adds the hook `harness newcmd`, a project at its tag `v1`, the cache and the head.
fn source_with_a_new_hook_command() -> (TempDir, TempDir, TempDir, String) {
    let (repo, _first, _second) = git_source();
    let root = git_project(&repo);
    let cache = tempdir().unwrap();
    update_in(
        Some(cache.path()),
        root.path(),
        Some("v1"),
        &SyncOptions::default(),
        &no_harness(),
    )
    .unwrap();
    put(
        repo.path(),
        &format!("entries/{}", entries::SETTINGS),
        r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "harness newcmd"}]}]}}"#,
    );
    git_in(repo.path(), &["add", "."]);
    git_in(repo.path(), &["commit", "--quiet", "-m", "new hook"]);
    let head = git_in(repo.path(), &["rev-parse", "HEAD"]);
    (repo, root, cache, head)
}

// spec: BIN-16
#[test]
fn update_writes_nothing_and_names_the_install_when_the_installed_harness_refuses_a_hook() {
    let (repo, root, cache, head) = source_with_a_new_hook_command();
    let bin = tempdir().unwrap();
    let old = harness_with_run(bin.path(), &bin.path().join("log"));
    let (config, lock) = (get(root.path(), CONFIG), get(root.path(), LOCK));

    let err = update_in(
        Some(cache.path()),
        root.path(),
        None,
        &SyncOptions::default(),
        &old,
    )
    .unwrap_err();
    let err = format!("{err:#}");
    assert!(err.contains("`harness newcmd`"), "{err}");
    let install = format!(
        "cargo install --git file://{} --rev {head} --locked",
        repo.path().display()
    );
    assert!(err.contains(&install), "{err}");
    assert_eq!(get(root.path(), CONFIG), config);
    assert_eq!(get(root.path(), LOCK), lock);

    // once the harness of the target is installed, the same update goes through
    let new = script(&bin.path().join("harness-new"), "exit 0\n");
    let done = update_in(
        Some(cache.path()),
        root.path(),
        None,
        &SyncOptions::default(),
        &new,
    )
    .unwrap();
    assert!(done.applied);
    assert_eq!(done.rev, head);
}

// spec: BIN-16
#[test]
fn update_dry_run_warns_of_a_hook_the_installed_harness_refuses() {
    let (_repo, root, cache, _head) = source_with_a_new_hook_command();
    let bin = tempdir().unwrap();
    let old = harness_with_run(bin.path(), &bin.path().join("log"));
    let options = SyncOptions {
        force: false,
        dry_run: true,
    };
    let done = update_in(Some(cache.path()), root.path(), None, &options, &old).unwrap();
    assert!(!done.applied);
    assert!(
        done.sync.warnings.iter().any(|w| w.starts_with("warning:")
            && w.contains("`harness newcmd`")
            && w.contains("cargo install")),
        "{:?}",
        done.sync.warnings
    );
}
