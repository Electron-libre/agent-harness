//! The format of the managed content: `sync` and `update` refuse a source of a format the running `cardinal` cannot
//! read (SYN-40), run as the binary on temp dirs.
#![cfg(unix)]

mod common;

use std::path::Path;

use common::{get, lock_of, project, put, snapshot, Home, CONFIG, LOCK};
use tempfile::{tempdir, TempDir};

/// The format the `cardinal` under test reads: the one of its own revision (SYN-43).
fn format() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../cardinal-format"))
        .unwrap()
        .trim()
        .to_string()
}

fn url(repo: &Path) -> String {
    format!("file://{}", repo.display())
}

// spec: SYN-40
#[test]
fn sync_refuses_a_source_of_a_newer_format_and_writes_nothing() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    put(source.path(), "files/a.txt", "a2");
    put(source.path(), "cardinal-format", "99\n");
    let before = snapshot(root.path());
    for args in [&["sync"][..], &["sync", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{run:?}");
        let install = format!(
            "cargo install --force --path {}/crates/cardinal --locked",
            source.path().display()
        );
        assert!(
            run.stderr.contains(&format!(
                "needs a cardinal that reads the format 99, this one reads up to {}: install it with `{install}`",
                format()
            )),
            "{run:?}"
        );
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: SYN-40
#[test]
fn sync_reads_a_source_of_the_same_or_an_older_format() {
    let home = Home::new();
    let (source, root) = project(&home);
    // the lower format first: a project the current one migrated refuses it (SYN-48)
    for declared in ["1".to_string(), format()] {
        put(source.path(), "cardinal-format", &format!("{declared}\n"));
        put(source.path(), "files/a.txt", &declared);
        home.ok(root.path(), &["sync"]);
        assert_eq!(get(root.path(), "a.txt"), Some(declared));
    }
}

// spec: SYN-40
#[test]
fn a_source_format_that_is_not_a_whole_number_is_an_error_naming_the_file() {
    let home = Home::new();
    let (source, root) = project(&home);
    put(source.path(), "cardinal-format", "two\n");
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    assert!(run.stderr.contains("cardinal-format"), "{run:?}");
    assert!(get(root.path(), LOCK).is_none());
}

// spec: SYN-40
#[test]
fn update_refuses_a_revision_of_a_newer_format_and_keeps_the_project_as_it_was() {
    let home = Home::new();
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
    put(repo.path(), "files/a.txt", "a2");
    put(repo.path(), "cardinal-format", "99\n");
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "newer format"]);
    let head = home.git(repo.path(), &["rev-parse", "HEAD"]);
    let before = snapshot(root.path());

    for args in [&["update"][..], &["update", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{run:?}");
        let install = format!(
            "cargo install --force --git {} --rev {head} --locked",
            url(repo.path())
        );
        assert!(
            run.stderr.contains(&format!(
                "needs a cardinal that reads the format 99, this one reads up to {}: install it with `{install}`",
                format()
            )),
            "{run:?}"
        );
        assert_eq!(snapshot(root.path()), before);
    }
}

/// A git source with one commit, holding `cardinal-format` when `declared` is given, a project pinned on it by
/// `update`, and the commit.
fn git_project(home: &Home, declared: Option<&str>) -> (TempDir, TempDir, String) {
    let repo = tempdir().unwrap();
    home.git(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), "files/a.txt", "a1");
    if let Some(declared) = declared {
        put(repo.path(), "cardinal-format", declared);
    }
    home.git(repo.path(), &["add", "."]);
    home.git(repo.path(), &["commit", "--quiet", "-m", "one"]);
    let commit = home.git(repo.path(), &["rev-parse", "HEAD"]);
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!("[harness]\nsource = \"{}\"\n", url(repo.path())),
    );
    (repo, root, commit)
}

// spec: SYN-40
#[test]
fn sync_of_an_unpinned_git_source_names_the_commit_it_checked_out() {
    let home = Home::new();
    let (repo, root, commit) = git_project(&home, Some("99\n"));
    let run = home.harness(root.path(), &["sync"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    let install = format!(
        "cargo install --force --git {} --rev {commit} --locked",
        url(repo.path())
    );
    assert!(
        run.stderr.contains(&format!("install it with `{install}`")),
        "{run:?}"
    );
    assert!(get(root.path(), LOCK).is_none());
}

/// The `format` lines of the lock of `root`.
fn format_lines(root: &Path) -> Vec<String> {
    get(root, lock_of(root))
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("format "))
        .map(String::from)
        .collect()
}

// spec: SYN-41
#[test]
fn the_lock_records_the_format_of_its_source_before_every_other_line() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    assert_eq!(format_lines(root.path()), ["format 1"]);
    put(source.path(), "cardinal-format", &format!("{}\n", format()));
    home.ok(root.path(), &["sync"]);
    assert_eq!(format_lines(root.path()), [format!("format {}", format())]);
    let lock = get(root.path(), lock_of(root.path())).unwrap();
    let first = lock.lines().find(|l| !l.starts_with('#')).unwrap();
    assert_eq!(first, format!("format {}", format()), "{lock}");
}

// spec: SYN-41
#[test]
fn a_lock_without_a_format_is_read_and_rewritten_with_one() {
    let home = Home::new();
    let (_source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    let lock = get(root.path(), LOCK).unwrap();
    let older: String = lock
        .lines()
        .filter(|l| !l.starts_with("format "))
        .map(|l| l.to_string() + "\n")
        .collect();
    put(root.path(), LOCK, &older);
    home.ok(root.path(), &["check"]);
    let run = home.ok(root.path(), &["sync"]);
    assert!(run.lines().is_empty(), "{run:?}");
    assert_eq!(get(root.path(), LOCK), Some(lock));
}

// spec: SYN-42
#[test]
fn check_sync_and_update_refuse_a_lock_of_a_newer_format_and_write_nothing() {
    let home = Home::new();
    let (source, root) = project(&home);
    home.ok(root.path(), &["sync"]);
    let lock = get(root.path(), LOCK).unwrap();
    // a newer format may write lines this one cannot parse, before its format line
    let newer = format!(
        "{}entry  {{\"ask\": \"Bash(x)\"}}\n../outside  h\nformat 99\n",
        lock.replace("format 1\n", "")
    );
    put(root.path(), LOCK, &newer);
    put(source.path(), "files/a.txt", "a2");
    let before = snapshot(root.path());
    let install = format!(
        "cargo install --force --path {}/crates/cardinal --locked",
        source.path().display()
    );
    let refusal = format!(
        "{LOCK} was synced from a source of the format 99, this one reads up to {}: install it with `{install}`",
        format()
    );
    for args in [&["check"][..], &["sync"], &["sync", "--dry-run"]] {
        let run = home.harness(root.path(), args);
        assert_eq!(run.code, Some(3), "{args:?} {run:?}");
        assert!(run.stderr.contains(&refusal), "{args:?} {run:?}");
        assert_eq!(snapshot(root.path()), before);
    }
}

// spec: SYN-42
#[test]
fn update_refuses_a_lock_of_a_newer_format_naming_the_pinned_revision() {
    let home = Home::new();
    let (repo, root, commit) = git_project(&home, None);
    home.ok(root.path(), &["update"]);
    let lock = get(root.path(), LOCK).unwrap();
    put(root.path(), LOCK, &lock.replace("format 1", "format 99"));
    let before = snapshot(root.path());

    let run = home.harness(root.path(), &["update"]);
    assert_eq!(run.code, Some(3), "{run:?}");
    let install = format!(
        "cargo install --force --git {} --rev {commit} --locked",
        url(repo.path())
    );
    assert!(
        run.stderr.contains(&format!("install it with `{install}`")),
        "{run:?}"
    );
    assert_eq!(snapshot(root.path()), before);
}
