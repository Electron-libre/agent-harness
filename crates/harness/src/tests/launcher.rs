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
