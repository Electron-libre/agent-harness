use std::fs;
use std::path::Path;

use tempfile::{tempdir, TempDir};

use super::*;

mod install_gate;
mod launcher;
mod retention;
mod run;
mod settings_entries;

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

// spec: SYN-11
#[test]
fn resolves_a_tag_a_branch_a_commit_and_the_default_to_a_full_hash() {
    let (repo, first, second) = git_source();
    let cache = tempdir().unwrap();
    let url = format!("file://{}", repo.path().display());
    let dir = fetch(cache.path(), &url).unwrap();
    assert_eq!(resolve_in(&dir, &url, Some("v1")).unwrap(), first);
    assert_eq!(resolve_in(&dir, &url, Some("main")).unwrap(), second);
    assert_eq!(resolve_in(&dir, &url, Some(&first)).unwrap(), first);
    assert_eq!(resolve_in(&dir, &url, None).unwrap(), second);
    assert!(resolve_in(&dir, &url, Some("nope")).is_err());
}

// spec: SYN-12
#[test]
fn set_rev_changes_only_the_rev_line() {
    let text = "# mine\n[harness]\nsource = \"x\" # why\nrev = \"old\"  # v0\n\n[project]\nrev = \"keep\"\n";
    let out = set_rev(text, "abc", "v1").unwrap();
    assert_eq!(
        out,
        "# mine\n[harness]\nsource = \"x\" # why\nrev = \"abc\"  # v1\n\n[project]\nrev = \"keep\"\n"
    );
}

// spec: SYN-12
#[test]
fn set_rev_inserts_the_rev_after_source_when_missing() {
    let text = "[harness]\nsource = \"x\"\n\n[project]\na = 1\n";
    let out = set_rev(text, "abc", "main").unwrap();
    assert_eq!(
        out,
        "[harness]\nsource = \"x\"\nrev = \"abc\"  # main\n\n[project]\na = 1\n"
    );
    assert_eq!(set_rev(&out, "abc", "main").unwrap(), out);
    assert!(set_rev("[project]\n", "abc", "main").is_err());
}

// spec: SYN-12
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

// spec: SYN-11
// spec: SYN-16
#[test]
fn update_pins_the_resolved_commit_then_syncs_and_reports_both_revisions() {
    let (repo, first, second) = git_source();
    let root = git_project(&repo);
    let cache = tempdir().unwrap();
    let opts = SyncOptions::default();

    let done = update_in(
        Some(cache.path()),
        root.path(),
        Some("v1"),
        &opts,
        &no_harness(),
    )
    .unwrap();
    assert_eq!(done.previous, None);
    assert_eq!(done.rev, first);
    assert_eq!(done.sync.outcome("a.txt"), Some(Outcome::Added));
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    let config = get(root.path(), CONFIG).unwrap();
    assert!(config.starts_with("# mine\n"));
    assert!(
        config.contains(&format!("rev = \"{first}\"  # v1\n")),
        "{config}"
    );

    let done = update_in(Some(cache.path()), root.path(), None, &opts, &no_harness()).unwrap();
    assert_eq!(done.previous.as_deref(), Some(first.as_str()));
    assert_eq!(done.rev, second);
    assert_eq!(done.sync.outcome("a.txt"), Some(Outcome::Updated));
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
    assert!(get(root.path(), CONFIG)
        .unwrap()
        .contains(&format!("rev = \"{second}\"")));

    // Already there: `rev` is rewritten as it was and nothing else happens.
    let before = get(root.path(), CONFIG);
    let done = update_in(Some(cache.path()), root.path(), None, &opts, &no_harness()).unwrap();
    assert_eq!(done.sync.outcome("a.txt"), Some(Outcome::Unchanged));
    assert_eq!(get(root.path(), CONFIG), before);
}

// spec: SYN-14
#[test]
fn update_refuses_a_local_directory_source_and_changes_nothing() {
    let (_source, root) = setup();
    let before = get(root.path(), CONFIG);
    let err = update(root.path(), None, &SyncOptions::default()).unwrap_err();
    assert!(format!("{err:#}").contains("local directory"), "{err:#}");
    assert_eq!(get(root.path(), CONFIG), before);
    assert!(get(root.path(), LOCK).is_none());
}

/// A project moved to `v1` by `update`, with its repository and cache.
fn updated_to_v1() -> (TempDir, String, TempDir, TempDir) {
    let (repo, first, _second) = git_source();
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
    (repo, first, root, cache)
}

// spec: SYN-13
#[test]
fn update_stops_on_a_conflict_unless_forced_and_writes_nothing() {
    let (_repo, first, root, cache) = updated_to_v1();
    put(root.path(), "a.txt", "local");
    let (config, lock) = (get(root.path(), CONFIG), get(root.path(), LOCK));

    let done = update_in(
        Some(cache.path()),
        root.path(),
        None,
        &SyncOptions::default(),
        &no_harness(),
    )
    .unwrap();
    assert!(!done.applied);
    assert_eq!(done.sync.conflicts(), 1);
    assert_eq!(done.previous.as_deref(), Some(first.as_str()));
    assert_eq!(get(root.path(), CONFIG), config);
    assert_eq!(get(root.path(), LOCK), lock);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("local"));

    let options = SyncOptions {
        force: true,
        dry_run: false,
    };
    let done = update_in(
        Some(cache.path()),
        root.path(),
        None,
        &options,
        &no_harness(),
    )
    .unwrap();
    assert!(done.applied);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
    assert!(!get(root.path(), CONFIG).unwrap().contains(&first));
}

// spec: SYN-13
#[test]
fn update_to_an_unknown_revision_or_an_unreadable_source_changes_nothing() {
    let (repo, _first, root, cache) = updated_to_v1();
    let (config, lock) = (get(root.path(), CONFIG), get(root.path(), LOCK));
    let options = SyncOptions::default();

    assert!(update_in(
        Some(cache.path()),
        root.path(),
        Some("nope"),
        &options,
        &no_harness()
    )
    .is_err());

    // A revision without files/: the source cannot be read as a harness.
    git_in(repo.path(), &["rm", "-rq", "files"]);
    git_in(repo.path(), &["commit", "--quiet", "-m", "empty"]);
    assert!(update_in(
        Some(cache.path()),
        root.path(),
        None,
        &options,
        &no_harness()
    )
    .is_err());

    let gone = git_project(&tempdir().unwrap());
    let gone_config = get(gone.path(), CONFIG);
    assert!(update_in(
        Some(cache.path()),
        gone.path(),
        None,
        &options,
        &no_harness()
    )
    .is_err());

    assert_eq!(get(root.path(), CONFIG), config);
    assert_eq!(get(root.path(), LOCK), lock);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    assert_eq!(get(gone.path(), CONFIG), gone_config);
}

// spec: SYN-15
#[test]
fn update_dry_run_reports_the_target_and_writes_nothing() {
    let (_repo, first, root, cache) = updated_to_v1();
    let (config, lock) = (get(root.path(), CONFIG), get(root.path(), LOCK));
    let options = SyncOptions {
        force: false,
        dry_run: true,
    };
    let done = update_in(
        Some(cache.path()),
        root.path(),
        None,
        &options,
        &no_harness(),
    )
    .unwrap();
    assert!(!done.applied);
    assert_ne!(done.rev, first);
    assert_eq!(done.sync.outcome("a.txt"), Some(Outcome::Updated));
    assert_eq!(get(root.path(), CONFIG), config);
    assert_eq!(get(root.path(), LOCK), lock);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
}

/// A source whose pinned commit `first` no branch or tag points to any more; `kept` tells whether the
/// source still holds it (else it was pruned).
fn rewritten_source(kept: bool) -> (TempDir, String) {
    let (repo, first, _second) = git_source();
    git_in(repo.path(), &["tag", "-d", "v1"]);
    git_in(
        repo.path(),
        &["checkout", "--quiet", "--orphan", "rewritten"],
    );
    put(repo.path(), "files/a.txt", "a3");
    git_in(repo.path(), &["add", "."]);
    git_in(repo.path(), &["commit", "--quiet", "-m", "three"]);
    git_in(repo.path(), &["branch", "-M", "main"]);
    git_in(repo.path(), &["reflog", "expire", "--expire=now", "--all"]);
    if !kept {
        git_in(repo.path(), &["gc", "--quiet", "--prune=now"]);
        // CI served the commit after the gc alone: fail here, not in the test, if it is still there
        assert!(
            !resolves(repo.path(), &first),
            "the commit is still in the source"
        );
    }
    (repo, first)
}

// spec: SYN-17
#[test]
fn a_pinned_hash_no_ref_points_to_any_more_is_fetched_by_hash() {
    let (repo, first) = rewritten_source(true);
    let cache = tempdir().unwrap();
    let url = format!("file://{}", repo.path().display());
    let dir = fetch(cache.path(), &url).unwrap();
    assert_eq!(resolve_in(&dir, &url, Some(&first)).unwrap(), first);

    let root = git_project(&repo);
    let config = get(root.path(), CONFIG).unwrap();
    put(root.path(), CONFIG, &format!("{config}rev = \"{first}\"\n"));
    let source = source_dir(Some(cache.path()), root.path(), None).unwrap();
    assert_eq!(get(&source, "files/a.txt").as_deref(), Some("a1"));
}

// spec: SYN-17
#[test]
fn a_pinned_hash_the_source_does_not_serve_fails_naming_update_and_changes_nothing() {
    let (repo, first) = rewritten_source(false);
    let root = git_project(&repo);
    let config = format!("{}rev = \"{first}\"\n", get(root.path(), CONFIG).unwrap());
    put(root.path(), CONFIG, &config);
    let cache = tempdir().unwrap();

    let err = source_dir(Some(cache.path()), root.path(), None).unwrap_err();
    let message = format!("{err:#}");
    assert!(message.contains("gone from the source"), "{message}");
    assert!(message.contains("harness update"), "{message}");
    assert_eq!(get(root.path(), CONFIG).as_deref(), Some(config.as_str()));
    assert!(get(root.path(), LOCK).is_none());

    // An abbreviated hash or a name keeps the plain error.
    let url = format!("file://{}", repo.path().display());
    let dir = fetch(cache.path(), &url).unwrap();
    for rev in [&first[..12], "nope"] {
        let err = resolve_in(&dir, &url, Some(rev)).unwrap_err();
        assert!(format!("{err:#}").contains("unknown revision"), "{err:#}");
    }
}

// spec: SYN-17
#[test]
fn only_a_full_hexadecimal_hash_is_fetched_by_hash() {
    assert!(is_full_hash(&"a1".repeat(20)));
    assert!(is_full_hash(&"A1".repeat(32)));
    assert!(!is_full_hash(&"a1".repeat(6)));
    assert!(!is_full_hash(&"g".repeat(40)));
    assert!(!is_full_hash("main"));
}
