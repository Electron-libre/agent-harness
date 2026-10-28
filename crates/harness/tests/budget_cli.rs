//! The budget of the fixed context of `check`, `sync` and `update`, run as the binary on temp dirs (SYN-38, SYN-39).
#![cfg(unix)]

mod common;

use std::path::Path;

use common::{get, project, put, Home, Run, CONFIG};
use tempfile::tempdir;

/// A skill whose front matter counts 1 + 9 = 10 bytes, with a body loaded on demand that counts for nothing.
const SKILL: &str = "---\nname: a\ndescription: \"012345678\"\n---\n\nA long body the session loads on demand only.\n";

/// The lines of stderr about the budget.
fn budget(run: &Run) -> Vec<&str> {
    run.stderr
        .lines()
        .filter(|l| l.contains("fixed_context_budget"))
        .collect()
}

fn over(total: u64, budget: u64, part: &str, bytes: u64) -> String {
    format!(
        "warning: the fixed context is {total} bytes, over the budget of {budget} (fixed_context_budget in .claude/harness.toml); the largest part is {part} ({bytes} bytes)"
    )
}

const NOT_A_NUMBER: &str =
    "warning: fixed_context_budget in .claude/harness.toml is not a whole number of bytes";

/// Sets `fixed_context_budget = <value>` in the `[project]` table of the project's `harness.toml`, replacing the
/// previous value.
fn set_budget(root: &Path, value: &str) {
    let config: String = get(root, CONFIG)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with("fixed_context_budget"))
        .map(|l| format!("{l}\n"))
        .collect();
    let line = format!("fixed_context_budget = {value}\n");
    let config = if config.contains("[project]\n") {
        config.replacen("[project]\n", &format!("[project]\n{line}"), 1)
    } else {
        format!("{config}\n[project]\n{line}")
    };
    put(root, CONFIG, &config);
}

/// A synced project of 30 bytes of `CLAUDE.md`, 20 of `AGENTS.md` and 10 of skill descriptions.
fn measured(home: &Home) -> (tempfile::TempDir, tempfile::TempDir) {
    let (source, root) = project(home);
    home.ok(root.path(), &["sync"]);
    put(root.path(), "CLAUDE.md", &"c".repeat(30));
    put(root.path(), "AGENTS.md", &"a".repeat(20));
    put(root.path(), ".claude/skills/a/SKILL.md", SKILL);
    (source, root)
}

// spec: SYN-38
#[test]
fn check_sync_and_dry_run_warn_once_when_the_fixed_context_exceeds_the_budget() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    set_budget(root.path(), "59");
    let warning = over(60, 59, "CLAUDE.md", 30);
    for args in [&["check"][..], &["sync"], &["sync", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(0), "{args:?}: {run:?}");
        assert_eq!(budget(&run), [warning.as_str()], "{args:?}: {run:?}");
    }
}

// spec: SYN-38
#[test]
fn no_warning_at_or_under_the_budget_nor_without_the_key() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    let run = home.ok(root.path(), &["check"]);
    assert!(budget(&run).is_empty(), "{run:?}");
    set_budget(root.path(), "60");
    let run = home.ok(root.path(), &["check"]);
    assert!(budget(&run).is_empty(), "{run:?}");
}

// spec: SYN-38
#[test]
fn the_largest_part_wins_its_ties_in_the_order_claude_agents_skills() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    set_budget(root.path(), "0");
    put(root.path(), "AGENTS.md", &"a".repeat(30));
    let run = home.ok(root.path(), &["check"]);
    assert_eq!(budget(&run), [over(70, 0, "CLAUDE.md", 30).as_str()]);
    std::fs::remove_file(root.path().join("CLAUDE.md")).unwrap();
    put(root.path(), ".claude/skills/b/SKILL.md", SKILL);
    put(root.path(), ".claude/skills/c/SKILL.md", SKILL);
    let run = home.ok(root.path(), &["check"]);
    assert_eq!(
        budget(&run),
        [over(60, 0, "AGENTS.md", 30).as_str()],
        "a missing CLAUDE.md counts 0, a tie goes to AGENTS.md"
    );
    put(root.path(), ".claude/skills/d/SKILL.md", SKILL);
    let run = home.ok(root.path(), &["check"]);
    assert_eq!(
        budget(&run),
        [over(70, 0, "skill descriptions", 40).as_str()]
    );
}

// spec: SYN-38
#[test]
fn a_symbolic_link_counts_as_the_file_it_points_to() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    set_budget(root.path(), "0");
    std::fs::remove_file(root.path().join("AGENTS.md")).unwrap();
    std::os::unix::fs::symlink("CLAUDE.md", root.path().join("AGENTS.md")).unwrap();
    let run = home.ok(root.path(), &["check"]);
    assert_eq!(budget(&run), [over(70, 0, "CLAUDE.md", 30).as_str()]);
}

// spec: SYN-38
#[test]
fn update_warns_once_judging_the_work_tree_it_leaves() {
    let home = Home::new();
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), "files/AGENTS.md", &"a".repeat(20));
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!(
            "[harness]\nsource = \"file://{}\"\n\n[project]\nfixed_context_budget = 25\n",
            repo.path().display()
        ),
    );
    put(root.path(), "CLAUDE.md", &"c".repeat(10));
    let run = home.ok(root.path(), &["update", "--dry-run"]);
    assert!(
        budget(&run).is_empty(),
        "the dry run judges the work tree as it is: {run:?}"
    );
    let run = home.ok(root.path(), &["update"]);
    assert_eq!(budget(&run), [over(30, 25, "AGENTS.md", 20).as_str()]);
}

// spec: SYN-39
#[test]
fn a_budget_that_is_not_a_whole_number_is_warned_of_once_without_failing() {
    let home = Home::new();
    let (_source, root) = measured(&home);
    for value in ["\"big\"", "-1", "1.5", "true"] {
        set_budget(root.path(), value);
        for args in [&["check"][..], &["sync"]] {
            let run = home.harness(root.path(), args);
            assert_eq!(run.code, Some(0), "{value} {args:?}: {run:?}");
            assert_eq!(budget(&run), [NOT_A_NUMBER], "{value} {args:?}: {run:?}");
        }
    }
}
