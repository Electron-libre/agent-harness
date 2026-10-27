use std::fs;
use std::path::Path;

use tempfile::{tempdir, TempDir};

use super::*;

mod install_gate;
mod launcher;
mod retention;
mod run;

fn put(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn get(dir: &Path, rel: &str) -> Option<String> {
    fs::read_to_string(dir.join(rel)).ok()
}

/// A `harness` that is not on the path: no hook command is looked at (BIN-15).
fn no_harness() -> std::ffi::OsString {
    "harness-not-on-the-path-of-the-tests".into()
}

/// A source with `files/a.txt`, `files/sub/b.txt` and `seed/s.txt`, and a project pointing at it.
fn setup() -> (TempDir, TempDir) {
    let source = tempdir().unwrap();
    put(source.path(), "files/a.txt", "a1");
    put(source.path(), "files/sub/b.txt", "b1");
    put(source.path(), "seed/s.txt", "seed1");
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!("[harness]\nsource = \"{}\"\n", source.path().display()),
    );
    (source, root)
}

fn run(root: &TempDir) -> Report {
    sync(root.path(), &SyncOptions::default()).unwrap()
}

fn git_in(dir: &Path, args: &[&str]) -> String {
    git_output(
        dir,
        &[
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ],
            args,
        ]
        .concat(),
    )
    .unwrap()
}

/// A git repository with `v1` on its first commit and `main` on its second; returns both hashes.
fn git_source() -> (TempDir, String, String) {
    let repo = tempdir().unwrap();
    git_in(repo.path(), &["init", "--quiet", "-b", "main"]);
    put(repo.path(), "files/a.txt", "a1");
    git_in(repo.path(), &["add", "."]);
    git_in(repo.path(), &["commit", "--quiet", "-m", "one"]);
    git_in(repo.path(), &["tag", "v1"]);
    let first = git_in(repo.path(), &["rev-parse", "HEAD"]);
    put(repo.path(), "files/a.txt", "a2");
    git_in(repo.path(), &["commit", "--quiet", "-am", "two"]);
    let second = git_in(repo.path(), &["rev-parse", "HEAD"]);
    (repo, first, second)
}

#[test]
fn set_rev_ignores_array_lines_and_refuses_what_it_cannot_edit() {
    let text = "[harness]\nextra = [\n  [\"a\"],\n]\nsource = \"x\"\nrev = \"old\"\n";
    let out = set_rev(text, "abc", "v1").unwrap();
    assert!(out.ends_with("rev = \"abc\"  # v1\n"), "{out}");
    assert_eq!(out.matches("rev =").count(), 1);
    assert!(set_rev(
        "[harness]\nsource = \"x\"\n\"rev\" = \"old\"\n",
        "abc",
        "v1"
    )
    .is_err());
    assert!(set_rev("[harness]\nsource = \"x\"\n", "abc", "a\nb").is_err());
}

/// A project whose source is the git repository `repo`.
fn git_project(repo: &TempDir) -> TempDir {
    let root = tempdir().unwrap();
    put(
        root.path(),
        CONFIG,
        &format!(
            "# mine\n[harness]\nsource = \"file://{}\"\n",
            repo.path().display()
        ),
    );
    root
}

#[test]
fn only_a_full_hexadecimal_hash_is_fetched_by_hash() {
    assert!(is_full_hash(&"a1".repeat(20)));
    assert!(is_full_hash(&"A1".repeat(32)));
    assert!(!is_full_hash(&"a1".repeat(6)));
    assert!(!is_full_hash(&"g".repeat(40)));
    assert!(!is_full_hash("main"));
}
