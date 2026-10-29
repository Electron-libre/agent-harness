//! The migrations a source declares (SYN-52, SYN-53), run as the binary on temp dirs: a project whose lock records a
//! format below the one of its source applies them.
#![cfg(unix)]

mod common;

use std::path::Path;

use common::{get, project, put, snapshot, Home, CONFIG, FORMAT, LOCK, SOURCE_FORMAT};
use tempfile::{tempdir, TempDir};

/// The rewrite of the seeded justfile line `check: check-old` into `check: check-new`.
const JUSTFILE_REWRITE: &str = r#"
[[rewrite]]
path = "justfile"
old = "check: check-old"
new = "check: check-new"
"#;

/// Make the lock of `root` record the format `format`, or none.
fn set_lock_format(root: &Path, format: Option<u32>) {
    let lock = get(root, LOCK).unwrap();
    let mut text: String = lock
        .lines()
        .filter(|l| !l.starts_with("format "))
        .flat_map(|l| [l, "\n"])
        .collect();
    if let Some(format) = format {
        text = format!("format {format}\n{text}");
    }
    put(root, LOCK, &text);
}

/// A project synced once, whose lock is then set back to the format 2, with a justfile of its own.
fn synced_at_format_2(home: &Home) -> (TempDir, TempDir) {
    let (source, root) = project(home);
    home.ok(root.path(), &["sync"]);
    set_lock_format(root.path(), Some(2));
    put(
        root.path(),
        "justfile",
        "check: check-old\nfmt: check: check-old\n",
    );
    (source, root)
}

// spec: SYN-52, SYN-53
#[test]
fn sync_rewrites_the_seeded_lines_of_a_migration_above_the_lock_once() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    put(source.path(), "migrations/3.toml", JUSTFILE_REWRITE);
    put(
        root.path(),
        "justfile",
        "check: check-old\ncheck: check-old fmt\nx check: check-old\ncheck: check-old",
    );

    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("justfile"), Some("rewritten"), "{run:?}");
    assert_eq!(
        get(root.path(), "justfile").unwrap(),
        "check: check-new\ncheck: check-old fmt\nx check: check-old\ncheck: check-new"
    );
    assert!(get(root.path(), LOCK).unwrap().contains("\nformat 3\n"));

    // the lock records the format 3 now: the migration is behind the project
    put(root.path(), "justfile", "check: check-old\n");
    let again = home.ok(root.path(), &["sync"]);
    assert_eq!(again.outcome("justfile"), None, "{again:?}");
    assert_eq!(get(root.path(), "justfile").unwrap(), "check: check-old\n");
}

// spec: SYN-52, SYN-53
#[test]
fn the_migrations_apply_in_increasing_format_from_a_lock_without_a_format() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    set_lock_format(root.path(), None);
    put(root.path(), "justfile", "one\n");
    let rewrite = |old: &str, new: &str| {
        format!("[[rewrite]]\npath = \"justfile\"\nold = \"{old}\"\nnew = \"{new}\"\n")
    };
    put(source.path(), "migrations/2.toml", &rewrite("one", "two"));
    put(source.path(), "migrations/3.toml", &rewrite("two", "three"));
    // above the format of the source, or below the one of the lock: not applied
    put(
        source.path(),
        "migrations/4.toml",
        &rewrite("three", "four"),
    );
    put(source.path(), "migrations/1.toml", &rewrite("one", "zero"));

    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(
        run.lines()
            .iter()
            .filter(|l| **l == "rewritten: justfile")
            .count(),
        1,
        "{run:?}"
    );
    assert_eq!(get(root.path(), "justfile").unwrap(), "three\n");
}

// spec: SYN-52
#[test]
fn a_project_without_a_lock_applies_no_migration() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "migrations/3.toml", JUSTFILE_REWRITE);
    put(root.path(), "justfile", "check: check-old\n");

    let run = home.ok(root.path(), &["sync"]);
    assert_eq!(run.outcome("justfile"), None, "{run:?}");
    assert_eq!(get(root.path(), "justfile").unwrap(), "check: check-old\n");
}

// spec: SYN-53
#[test]
fn sync_dry_run_reports_the_rewrites_and_writes_nothing() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    put(source.path(), "migrations/3.toml", JUSTFILE_REWRITE);
    let before = snapshot(root.path());

    let run = home.ok(root.path(), &["sync", "--dry-run"]);
    assert_eq!(run.outcome("justfile"), Some("rewritten"), "{run:?}");
    assert_eq!(snapshot(root.path()), before);
}

// spec: SYN-53
#[test]
fn a_rewrite_of_a_managed_file_the_lock_a_path_outside_or_a_missing_file_is_left_alone() {
    let home = Home::new();
    let (source, root) = synced_at_format_2(&home);
    let rewrite =
        |path: &str| format!("[[rewrite]]\npath = \"{path}\"\nold = \"a1\"\nnew = \"x\"\n");
    let text: String = ["a.txt", LOCK, "../out.txt", "missing.txt"]
        .iter()
        .map(|p| rewrite(p))
        .collect();
    put(source.path(), "migrations/3.toml", &text);
    let outside = root.path().parent().unwrap().join("out.txt");
    let had_outside = outside.exists();

    let run = home.ok(root.path(), &["sync"]);
    assert!(!run.stdout.contains("rewritten"), "{run:?}");
    assert_eq!(get(root.path(), "a.txt").unwrap(), "a1");
    assert!(!root.path().join("missing.txt").exists());
    assert_eq!(outside.exists(), had_outside);
}

// spec: SYN-52
#[test]
fn a_migration_of_the_wrong_shape_fails_the_sync_naming_it_and_writes_nothing() {
    for text in [
        "names = \"old\"\n",
        "[[names]]\nold = \"x\"\n",
        "[[rewrite]]\npath = \"justfile\"\nold = \"\"\nnew = \"y\"\n",
        "[[rewrites]]\npath = \"justfile\"\nold = \"x\"\nnew = \"y\"\n",
        "not toml = [\n",
    ] {
        let home = Home::new();
        let (source, root) = synced_at_format_2(&home);
        put(source.path(), "files/a.txt", "a2");
        put(source.path(), "migrations/3.toml", text);
        let before = snapshot(root.path());
        for args in [&["sync"][..], &["sync", "--dry-run"]] {
            let run = home.harness(root.path(), args);
            assert_eq!(run.code, Some(3), "{text}: {run:?}");
            assert!(run.stderr.contains("migrations/3.toml"), "{text}: {run:?}");
            assert_eq!(snapshot(root.path()), before, "{text}");
        }
    }
}

/// A git source of the format 3 with `files/a.txt`, whose head declares `migration` as `migrations/3.toml`.
fn git_source(home: &Home, migration: &str) -> TempDir {
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), SOURCE_FORMAT, FORMAT);
    put(repo.path(), "files/a.txt", "a1");
    put(repo.path(), "migrations/3.toml", migration);
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    repo
}

/// A project of `repo` updated once, whose lock is then set back to the format 2.
fn updated_at_format_2(home: &Home, repo: &Path) -> TempDir {
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!(
            "# old header\n[cardinal]\nsource = \"file://{}\"\n",
            repo.display()
        ),
    );
    home.ok(root.path(), &["update"]);
    set_lock_format(root.path(), Some(2));
    root
}

// spec: SYN-53
#[test]
fn update_rewrites_the_configuration_and_keeps_the_rev_it_writes() {
    let home = Home::new();
    let header = "[[rewrite]]\npath = \".claude/cardinal.toml\"\nold = \"# old header\"\nnew = \"# new header\"\n";
    let repo = git_source(&home, header);
    let root = updated_at_format_2(&home, repo.path());

    let run = home.ok(root.path(), &["update"]);
    assert_eq!(run.outcome(CONFIG), Some("rewritten"), "{run:?}");
    let config = get(root.path(), CONFIG).unwrap();
    assert!(config.starts_with("# new header\n[cardinal]\n"), "{config}");
    let head = home.git(repo.path(), &["rev-parse", "HEAD"]);
    assert!(config.contains(&format!("rev = \"{head}\"")), "{config}");
}

// spec: SYN-53
#[test]
fn update_stopped_by_a_conflict_rewrites_nothing() {
    let home = Home::new();
    let repo = git_source(&home, JUSTFILE_REWRITE);
    let root = updated_at_format_2(&home, repo.path());
    put(root.path(), "justfile", "check: check-old\n");
    put(root.path(), "a.txt", "mine");
    put(repo.path(), "files/a.txt", "a2");
    home.git(repo.path(), &["commit", "--quiet", "-am", "two"]);
    let before = snapshot(root.path());

    let run = home.harness(root.path(), &["update"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_eq!(snapshot(root.path()), before);
}
