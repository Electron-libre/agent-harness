//! The names of the files `cardinal` keeps in a project, which follow the format of its source, and the migration of
//! a project that still has the old ones (SYN-44 to SYN-47), run as the binary on temp dirs.
#![cfg(unix)]

mod common;

use std::path::Path;

use common::{get, put, snapshot, Home, CONFIG as NEW_CONFIG, LOCK as NEW_LOCK};
use tempfile::{tempdir, TempDir};

/// The configuration and the lock before the format 3.
const CONFIG: &str = ".claude/harness.toml";
const LOCK: &str = ".claude/harness.lock";
const CONFIG_RENAMED: &str = "renamed: .claude/harness.toml -> .claude/cardinal.toml";
const LOCK_RENAMED: &str = "renamed: .claude/harness.lock -> .claude/cardinal.lock";

/// A source of the format 1 with `files/a.txt`, `files/sub/b.txt` and `seed/s.txt`, and a project of the old names
/// initialised on it.
fn project(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = common::project(home);
    std::fs::remove_file(source.path().join(common::SOURCE_FORMAT)).unwrap();
    std::fs::remove_file(root.path().join(NEW_CONFIG)).unwrap();
    home.ok(root.path(), &["init", source.path().to_str().unwrap()]);
    (source, root)
}

fn url(repo: &Path) -> String {
    format!("file://{}", repo.display())
}

/// A project of the old names synced from a source of the format 1 that manages `harness.just`, its `justfile`
/// importing it twice among other lines; the source then moves to the format 3 and manages `cardinal.just` instead.
fn old_project(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = project(home);
    put(source.path(), "files/harness.just", "x:\n");
    home.ok(root.path(), &["sync"]);
    let config = get(root.path(), CONFIG).unwrap();
    put(
        root.path(),
        CONFIG,
        &format!("# mine\n{}  # kept\n[project]\nk = 1\n", config.trim_end()).replacen(
            "[harness]",
            "[harness]  # table",
            1,
        ),
    );
    put(
        root.path(),
        "justfile",
        "import 'harness.just'\n  import? \"harness.just\"  # x\n# import 'harness.just'\nother 'harness.just'\n",
    );
    put(source.path(), "cardinal-format", "3\n");
    std::fs::remove_file(source.path().join("files/harness.just")).unwrap();
    put(source.path(), "files/cardinal.just", "x:\n");
    put(source.path(), "seed/.claude/cardinal.toml", "[project]\n");
    (source, root)
}

// spec: SYN-44
#[test]
fn sync_keeps_the_old_names_below_the_format_3_and_writes_the_new_ones_from_it() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "cardinal-format", "2\n");
    home.ok(root.path(), &["sync"]);
    assert!(get(root.path(), LOCK).unwrap().contains("format 2\n"));
    assert!(get(root.path(), NEW_LOCK).is_none());
    assert!(get(root.path(), NEW_CONFIG).is_none());

    let fresh = tempdir().unwrap();
    put(source.path(), "cardinal-format", "3\n");
    put(
        fresh.path(),
        NEW_CONFIG,
        &format!("[cardinal]\nsource = \"{}\"\n", source.path().display()),
    );
    home.ok(fresh.path(), &["sync"]);
    assert!(get(fresh.path(), NEW_LOCK).unwrap().contains("format 3\n"));
    assert!(get(fresh.path(), LOCK).is_none());
    assert!(get(fresh.path(), CONFIG).is_none());
    home.ok(fresh.path(), &["check"]);
}

// spec: SYN-45
#[test]
fn sync_migrates_the_old_names_and_a_dry_run_only_reports_it() {
    let home = Home::new();
    let (_source, root) = old_project(&home);
    let config = get(root.path(), CONFIG).unwrap();
    let before = snapshot(root.path());

    let dry = home.ok(root.path(), &["sync", "--dry-run"]);
    for line in [CONFIG_RENAMED, LOCK_RENAMED, "updated: justfile"] {
        assert!(dry.lines().contains(&line), "{line}: {dry:?}");
    }
    assert_eq!(dry.outcome(NEW_CONFIG), None, "{dry:?}");
    assert_eq!(snapshot(root.path()), before);

    let run = home.ok(root.path(), &["sync"]);
    for line in [CONFIG_RENAMED, LOCK_RENAMED, "updated: justfile"] {
        assert!(run.lines().contains(&line), "{line}: {run:?}");
    }
    assert_eq!(run.outcome("harness.just"), Some("removed"), "{run:?}");
    assert_eq!(run.outcome("cardinal.just"), Some("added"), "{run:?}");
    assert_eq!(run.outcome(NEW_CONFIG), None, "{run:?}");
    assert_eq!(
        get(root.path(), NEW_CONFIG),
        Some(config.replacen("[harness]  # table", "[cardinal]  # table", 1))
    );
    assert!(get(root.path(), CONFIG).is_none());
    assert!(get(root.path(), LOCK).is_none());
    assert!(get(root.path(), NEW_LOCK).unwrap().contains("format 3\n"));
    assert_eq!(
        get(root.path(), "justfile").as_deref(),
        Some("import 'cardinal.just'\n  import? \"cardinal.just\"  # x\n# import 'harness.just'\nother 'harness.just'\n")
    );

    home.ok(root.path(), &["check"]);
    let again = home.ok(root.path(), &["sync"]);
    assert!(again.lines().is_empty(), "{again:?}");
}

// spec: SYN-45
#[test]
fn a_migration_without_a_lock_nor_a_justfile_renames_the_configuration_only() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "cardinal-format", "3\n");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(
        run.lines()
            .iter()
            .filter(|l| l.starts_with("renamed: "))
            .count(),
        1
    );
    assert!(run.lines().contains(&CONFIG_RENAMED), "{run:?}");
    assert!(!run.stdout.contains("justfile"), "{run:?}");
    assert!(get(root.path(), NEW_LOCK).is_some());
    assert!(get(root.path(), "justfile").is_none());
}

// spec: SYN-45
#[test]
fn a_table_that_cannot_be_renamed_line_by_line_fails_the_migration_changing_nothing() {
    let home = Home::new();
    let (source, root) = old_project(&home);
    put(
        root.path(),
        CONFIG,
        &format!("harness.source = \"{}\"\n", source.path().display()),
    );
    let before = snapshot(root.path());
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("[cardinal]"), "{run:?}");
    assert_eq!(snapshot(root.path()), before);
}

/// A git source of the format 1 that manages `harness.just`, a project pinned on it by `update`, and its commit.
fn git_project(home: &Home) -> (TempDir, TempDir, String) {
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), "files/harness.just", "x:\n");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    let commit = home.git(repo.path(), &["rev-parse", "HEAD"]);
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!(
            "[harness]\nsource = \"{}\"\n[project]\nk = 1\n",
            url(repo.path())
        ),
    );
    put(root.path(), "justfile", "import 'harness.just'\n");
    home.ok(root.path(), &["update"]);
    (repo, root, commit)
}

// spec: SYN-45
#[test]
fn update_to_a_revision_of_the_format_3_migrates_and_pins_it_in_the_new_configuration() {
    let home = Home::new();
    let (repo, root, old) = git_project(&home);
    home.git(
        repo.path(),
        &["mv", "files/harness.just", "files/cardinal.just"],
    );
    put(repo.path(), "cardinal-format", "3\n");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "two"]);
    let new = home.git(repo.path(), &["rev-parse", "HEAD"]);

    let before = snapshot(root.path());
    let dry = home.ok(root.path(), &["update", "--dry-run"]);
    assert!(dry.lines().contains(&CONFIG_RENAMED), "{dry:?}");
    assert_eq!(snapshot(root.path()), before);

    let run = home.ok(root.path(), &["update"]);
    for line in [CONFIG_RENAMED, LOCK_RENAMED, "updated: justfile"] {
        assert!(run.lines().contains(&line), "{line}: {run:?}");
    }
    assert!(run.stdout.contains(&old), "{run:?}");
    assert_eq!(
        get(root.path(), NEW_CONFIG),
        Some(format!(
            "[cardinal]\nsource = \"{}\"\nrev = \"{new}\"  # default branch\n[project]\nk = 1\n",
            url(repo.path())
        ))
    );
    assert!(get(root.path(), CONFIG).is_none());
    assert!(get(root.path(), LOCK).is_none());
    assert_eq!(
        get(root.path(), "justfile").as_deref(),
        Some("import 'cardinal.just'\n")
    );
    assert!(get(root.path(), "cardinal.just").is_some());
    assert!(get(root.path(), "harness.just").is_none());
}

// spec: SYN-46
#[test]
fn init_names_the_configuration_after_the_format_of_its_source() {
    let home = Home::new();
    let source = tempdir().unwrap();
    put(source.path(), "cardinal-format", "3\n");
    put(
        source.path(),
        "seed/.claude/cardinal.toml",
        "[project]\nname = \"new\"\n",
    );
    put(
        source.path(),
        "seed/.claude/harness.toml",
        "[project]\nname = \"old\"\n",
    );
    let root = tempdir().unwrap();
    home.ok(root.path(), &["init", source.path().to_str().unwrap()]);
    let text = get(root.path(), NEW_CONFIG).unwrap();
    let expected = format!("[cardinal]\nsource = \"{}\"\n", source.path().display());
    assert!(text.starts_with(&expected), "{text}");
    assert!(text.contains("name = \"new\""), "{text}");
    assert!(get(root.path(), CONFIG).is_none());

    let run = home.harness(root.path(), &["init", "elsewhere"]);
    assert_eq!(run.code, Some(3), "{run:?}");

    // an old configuration is refused too, whatever the format of the new source
    let old = tempdir().unwrap();
    put(old.path(), CONFIG, "[harness]\nsource = \"x\"\n");
    let run = home.harness(old.path(), &["init", source.path().to_str().unwrap()]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(get(old.path(), NEW_CONFIG).is_none());

    let older = tempdir().unwrap();
    put(source.path(), "cardinal-format", "2\n");
    home.ok(older.path(), &["init", source.path().to_str().unwrap()]);
    assert!(get(older.path(), CONFIG)
        .unwrap()
        .contains("name = \"old\""));
    assert!(get(older.path(), NEW_CONFIG).is_none());
}

// spec: SYN-47
#[test]
fn the_new_configuration_and_lock_are_read_before_the_old_ones() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "cardinal-format", "3\n");
    home.ok(root.path(), &["sync"]);
    // stale files of the old names point elsewhere and record nothing: the new ones are read
    put(root.path(), CONFIG, "[harness]\nsource = \"/nowhere\"\n");
    put(root.path(), LOCK, "format 1\n");
    std::fs::remove_file(root.path().join("a.txt")).unwrap();
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert!(run.stdout.contains("missing: a.txt"), "{run:?}");
    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("a.txt"), Some("added"), "{run:?}");
    assert!(
        get(root.path(), LOCK).is_none(),
        "the stale old lock is dropped"
    );
    assert_eq!(
        get(root.path(), CONFIG).as_deref(),
        Some("[harness]\nsource = \"/nowhere\"\n")
    );
}

// spec: SYN-48
#[test]
fn a_migrated_project_refuses_a_source_below_the_format_3_and_writes_nothing() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "cardinal-format", "3\n");
    home.ok(root.path(), &["sync"]);
    put(source.path(), "cardinal-format", "2\n");
    put(source.path(), "files/a.txt", "a2");
    let before = snapshot(root.path());
    for args in [&["sync"][..], &["sync", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{args:?} {run:?}");
        assert!(
            run.stderr.contains(&format!(
                "{NEW_CONFIG} needs a source of the format 3 or higher, {} has the format 2",
                source.path().display()
            )),
            "{run:?}"
        );
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: SYN-45
#[test]
fn the_migration_rewrites_a_justfile_of_another_name() {
    let home = Home::new();
    let (_source, root) = old_project(&home);
    let text = get(root.path(), "justfile").unwrap();
    std::fs::remove_file(root.path().join("justfile")).unwrap();
    put(root.path(), ".justfile", &text);
    let run = home.ok(root.path(), &["sync"]);
    assert!(run.lines().contains(&"updated: .justfile"), "{run:?}");
    assert!(get(root.path(), ".justfile")
        .unwrap()
        .starts_with("import 'cardinal.just'\n"));
    assert!(get(root.path(), "justfile").is_none());
}

// spec: SYN-47
#[test]
fn check_of_a_project_never_synced_names_the_lock_of_its_configuration() {
    let home = Home::new();
    let (_source, root) = project(&home);
    let run = home.harness(root.path(), &["check"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains(&format!("no {LOCK}")), "{run:?}");
}
