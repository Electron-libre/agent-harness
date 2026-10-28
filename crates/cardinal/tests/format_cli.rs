//! The format of the managed content: `sync` and `update` refuse a source of a format the running `cardinal` cannot
//! read (SYN-40), run as the binary on temp dirs.
#![cfg(unix)]

mod common;

use std::path::Path;

use common::{get, project, put, snapshot, Home, CONFIG, LOCK};
use tempfile::tempdir;

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
    for declared in [format(), "1".to_string()] {
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
