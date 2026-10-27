//! The hook commands the installed `harness` must accept (BIN-15, BIN-16), run as the binary with a fake installed
//! `harness` first on the path.
#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{get, project, put, Home, CONFIG, LOCK};
use tempfile::{tempdir, TempDir};

const SETTINGS: &str = ".claude/settings.json";

/// Make the installed `harness` of `home` one that knows only the subcommand `run`, logging each call to `<home>/log`.
fn install_harness_with_run(home: &Home) {
    home.program(
        "harness",
        &format!(
            "echo \"$*\" >> '{}/log'\n[ \"$1\" = help ] && [ \"$2\" = run ]\n",
            home.path().display()
        ),
    );
}

const HOOKS: &str = r#"{"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [
    {"type": "command", "command": "harness run review-gate check"},
    {"type": "command", "command": "harness 'run' quoted"},
    {"type": "command", "command": "harness newcmd --flag"},
    {"type": "command", "command": "sh -c 'harness newcmd'"}
]}], "Stop": [{"hooks": [{"type": "command", "command": "harness newcmd again"}]}]}}"#;

/// A synced project whose settings hold `HOOKS`.
fn project_with_hooks(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = project(home);
    home.ok(root.path(), &["sync"]);
    put(root.path(), SETTINGS, HOOKS);
    (source, root)
}

// spec: BIN-15
#[test]
fn check_fails_on_each_hook_command_whose_subcommand_the_installed_harness_refuses() {
    let home = Home::new();
    let (source, root) = project_with_hooks(&home);
    install_harness_with_run(&home);

    let run = home.harness(root.path(), &["check"]);

    assert_eq!(run.code, Some(1), "{run:?}");
    let problems = run.lines();
    assert_eq!(problems.len(), 2, "{run:?}");
    assert!(problems[0].contains("`harness newcmd --flag`"), "{run:?}");
    assert!(problems[1].contains("`harness newcmd again`"), "{run:?}");
    // the install of the project's source, run from its root
    let install = format!(
        "cargo install --path {}/crates/harness --locked",
        source.path().display()
    );
    assert!(problems.iter().all(|p| p.contains(&install)), "{run:?}");
    // each subcommand is asked once
    assert_eq!(get(home.path(), "log").unwrap(), "help run\nhelp newcmd\n");
}

/// The `PATH` of the binary without any `harness`: the home's own `bin`, the `harness` removed, and the directories of
/// the caller's `PATH` that hold none.
fn path_without_harness(home: &Home) -> std::ffi::OsString {
    fs::remove_file(home.path().join("bin/harness")).unwrap();
    let caller = std::env::var_os("PATH").unwrap_or_default();
    let dirs = std::iter::once(home.path().join("bin")).chain(
        std::env::split_paths(&caller).filter(|dir: &PathBuf| !dir.join("harness").exists()),
    );
    std::env::join_paths(dirs).unwrap()
}

// spec: BIN-15
#[test]
fn check_looks_at_nothing_without_a_harness_on_the_path() {
    let home = Home::new();
    let (_source, root) = project_with_hooks(&home);
    let mut cmd = home.command();
    cmd.env("PATH", path_without_harness(&home))
        .args(["check", "--root"])
        .arg(root.path());

    let run = common::output(cmd, "");

    assert_eq!(run.code, Some(0), "{run:?}");
    assert_eq!(run.stdout, "");
}

/// A git source whose head adds the hooks `harness newcmd` and `harness newcmd other`, a project moved to its tag
/// `v1`, and the head.
fn source_with_a_new_hook_command(home: &Home) -> (TempDir, TempDir, String) {
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), "files/a.txt", "a1");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    home.git(repo.path(), &["tag", "v1"]);
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!("[harness]\nsource = \"{}\"\n", url(repo.path())),
    );
    home.ok(root.path(), &["update", "--rev", "v1"]);
    put(
        repo.path(),
        &format!("entries/{SETTINGS}"),
        r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "harness newcmd"},
            {"type": "command", "command": "harness newcmd other"}]}]}}"#,
    );
    put(repo.path(), "files/a.txt", "a2");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "new hook"]);
    let head = home.git(repo.path(), &["rev-parse", "HEAD"]);
    (repo, root, head)
}

fn url(repo: &Path) -> String {
    format!("file://{}", repo.display())
}

// spec: BIN-16
#[test]
fn update_writes_nothing_and_names_the_install_when_the_installed_harness_refuses_a_hook() {
    let home = Home::new();
    let (repo, root, head) = source_with_a_new_hook_command(&home);
    install_harness_with_run(&home);
    let (config, lock) = (get(root.path(), CONFIG), get(root.path(), LOCK));

    let run = home.harness(root.path(), &["update"]);

    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("`harness newcmd`"), "{run:?}");
    assert!(run.stderr.contains("`harness newcmd other`"), "{run:?}");
    let install = format!(
        "cargo install --git {} --rev {head} --locked",
        url(repo.path())
    );
    // one install for all the hooks
    assert_eq!(run.stderr.matches(&install).count(), 1, "{run:?}");
    assert_eq!(get(root.path(), CONFIG), config);
    assert_eq!(get(root.path(), LOCK), lock);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    assert!(get(root.path(), SETTINGS).is_none());

    // once the harness of the target is installed, the same update goes through
    home.program("harness", "exit 0\n");
    let run = home.ok(root.path(), &["update"]);
    assert!(run.stdout.starts_with("rev: "), "{run:?}");
    assert!(get(root.path(), CONFIG).unwrap().contains(&head), "{run:?}");
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
}

// spec: BIN-16
#[test]
fn update_dry_run_warns_of_a_hook_the_installed_harness_refuses() {
    let home = Home::new();
    let (_repo, root, _head) = source_with_a_new_hook_command(&home);
    install_harness_with_run(&home);
    let config = get(root.path(), CONFIG);

    let run = home.ok(root.path(), &["update", "--dry-run"]);

    let warned = |text: &str| {
        run.stderr
            .lines()
            .any(|w| w.starts_with("warning:") && w.contains(text))
    };
    assert!(
        warned("`harness newcmd`") && warned("cargo install"),
        "{run:?}"
    );
    assert_eq!(get(root.path(), CONFIG), config);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
}
