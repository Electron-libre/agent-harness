use std::fs;
use std::path::Path;

use tempfile::{tempdir, TempDir};

use super::*;

fn put(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn get(dir: &Path, rel: &str) -> Option<String> {
    fs::read_to_string(dir.join(rel)).ok()
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

// spec: SYN-1
#[test]
fn copies_every_managed_file_to_the_same_path() {
    let (_source, root) = setup();
    let report = run(&root);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    assert_eq!(get(root.path(), "sub/b.txt").as_deref(), Some("b1"));
    assert_eq!(report.outcome("a.txt"), Some(Outcome::Added));
}

// spec: SYN-2
#[test]
fn overwrites_an_untouched_file_when_the_source_changed() {
    let (source, root) = setup();
    run(&root);
    put(source.path(), "files/a.txt", "a2");
    let report = run(&root);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
    assert_eq!(report.outcome("a.txt"), Some(Outcome::Updated));
}

// spec: SYN-3
#[test]
fn keeps_a_locally_modified_file_and_fails_without_force() {
    let (source, root) = setup();
    run(&root);
    put(root.path(), "a.txt", "local");
    put(source.path(), "files/a.txt", "a2");
    let report = run(&root);
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("local"));
    assert_eq!(report.outcome("a.txt"), Some(Outcome::Conflict));
    assert_eq!(report.conflicts(), 1);
    // The conflict survives the next run: the lock still holds the last synced hash.
    assert_eq!(run(&root).outcome("a.txt"), Some(Outcome::Conflict));
    let forced = sync(
        root.path(),
        &SyncOptions {
            force: true,
            dry_run: false,
        },
    )
    .unwrap();
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
    assert_eq!(forced.conflicts(), 0);
}

// spec: SYN-3
#[test]
fn a_differing_file_that_was_never_synced_is_a_conflict() {
    let (_source, root) = setup();
    put(root.path(), "a.txt", "mine");
    let report = run(&root);
    assert_eq!(report.outcome("a.txt"), Some(Outcome::Conflict));
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("mine"));
}

// spec: SYN-4
#[test]
fn seeds_only_what_is_missing_and_never_touches_it_again() {
    let (source, root) = setup();
    assert_eq!(run(&root).outcome("s.txt"), Some(Outcome::Seeded));
    put(root.path(), "s.txt", "edited");
    put(source.path(), "seed/s.txt", "seed2");
    let report = run(&root);
    assert_eq!(get(root.path(), "s.txt").as_deref(), Some("edited"));
    assert_eq!(report.outcome("s.txt"), None);
    let locked = fs::read_to_string(root.path().join(LOCK)).unwrap();
    assert!(!locked.contains("s.txt"), "a seed is not managed: {locked}");
}

// spec: SYN-5
#[test]
fn removes_a_dropped_file_unless_it_was_modified() {
    let (source, root) = setup();
    run(&root);
    put(root.path(), "sub/b.txt", "local");
    fs::remove_file(source.path().join("files/a.txt")).unwrap();
    fs::remove_file(source.path().join("files/sub/b.txt")).unwrap();
    let report = run(&root);
    assert_eq!(report.outcome("a.txt"), Some(Outcome::Removed));
    assert!(get(root.path(), "a.txt").is_none());
    assert_eq!(report.outcome("sub/b.txt"), Some(Outcome::Kept));
    assert_eq!(get(root.path(), "sub/b.txt").as_deref(), Some("local"));
    assert!(run(&root)
        .entries
        .iter()
        .all(|(_, o)| *o == Outcome::Unchanged || *o == Outcome::Seeded));
}

// spec: SYN-6
#[test]
fn a_second_sync_reports_nothing_and_changes_nothing() {
    let (_source, root) = setup();
    run(&root);
    let lock = get(root.path(), LOCK);
    let report = run(&root);
    assert!(
        report.entries.iter().all(|(_, o)| *o == Outcome::Unchanged),
        "{:?}",
        report.entries
    );
    assert_eq!(get(root.path(), LOCK), lock);
}

// spec: SYN-6
#[test]
fn an_identical_file_missing_from_the_lock_is_locked_and_unchanged() {
    let (_source, root) = setup();
    put(root.path(), "a.txt", "a1");
    assert_eq!(run(&root).outcome("a.txt"), Some(Outcome::Unchanged));
    assert!(get(root.path(), LOCK).unwrap().contains("a.txt"));
}

// spec: SYN-7
#[test]
fn dry_run_reports_and_writes_nothing() {
    let (_source, root) = setup();
    let report = sync(
        root.path(),
        &SyncOptions {
            force: false,
            dry_run: true,
        },
    )
    .unwrap();
    assert_eq!(report.outcome("a.txt"), Some(Outcome::Added));
    assert!(get(root.path(), "a.txt").is_none());
    assert!(get(root.path(), "s.txt").is_none());
    assert!(get(root.path(), LOCK).is_none());
}

// spec: SYN-8
#[test]
fn check_lists_modified_and_deleted_managed_files_only() {
    let (_source, root) = setup();
    run(&root);
    assert!(check(root.path()).unwrap().is_empty());
    put(root.path(), "a.txt", "local");
    fs::remove_file(root.path().join("sub/b.txt")).unwrap();
    put(root.path(), "s.txt", "a seed may change freely");
    assert_eq!(
        check(root.path()).unwrap(),
        ["modified: a.txt", "missing: sub/b.txt"]
    );
}

// spec: SYN-8
#[test]
fn check_fails_when_the_project_was_never_synced() {
    let (_source, root) = setup();
    assert!(check(root.path()).is_err());
}

// spec: SYN-9
#[test]
fn init_writes_the_config_with_the_source_defaults_and_refuses_to_overwrite() {
    let source = tempdir().unwrap();
    put(
        source.path(),
        "seed/.claude/harness.toml",
        "[project]\nname = \"x\"\n",
    );
    let root = tempdir().unwrap();
    init(root.path(), &source.path().to_string_lossy(), None).unwrap();
    let text = get(root.path(), CONFIG).unwrap();
    assert!(text.starts_with("[harness]\nsource = "), "{text}");
    assert!(text.contains("[project]\nname = \"x\""), "{text}");
    assert!(init(root.path(), "elsewhere", None).is_err());
}

// spec: SYN-10
#[test]
fn a_source_without_files_fails_and_deletes_nothing() {
    let (source, root) = setup();
    run(&root);
    fs::remove_dir_all(source.path().join("files")).unwrap();
    assert!(sync(root.path(), &SyncOptions::default()).is_err());
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
    assert!(get(root.path(), LOCK).unwrap().contains("a.txt"));
}

// spec: SYN-10
#[test]
fn a_lock_path_outside_the_project_is_refused() {
    let (_source, root) = setup();
    run(&root);
    let outside = tempdir().unwrap();
    put(outside.path(), "x", "precious");
    let lock = get(root.path(), LOCK).unwrap();
    let evil = outside.path().join("x");
    put(
        root.path(),
        LOCK,
        &format!("{lock}{}  {}\n", hash(b"precious"), evil.display()),
    );
    assert!(sync(root.path(), &SyncOptions::default()).is_err());
    assert!(check(root.path()).is_err());
    assert!(evil.exists());
}

// spec: SYN-10
#[test]
fn an_unreadable_lock_is_an_error_not_a_first_sync() {
    let (_source, root) = setup();
    run(&root);
    fs::write(root.path().join(LOCK), [0xff, 0xfe, 0x00]).unwrap();
    assert!(sync(root.path(), &SyncOptions::default()).is_err());
    assert!(check(root.path()).is_err());
}

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::{symlink, PermissionsExt};

    use super::*;

    fn mode(dir: &Path, rel: &str) -> u32 {
        fs::metadata(dir.join(rel)).unwrap().permissions().mode() & 0o111
    }

    // spec: SYN-1
    #[test]
    fn keeps_the_executable_bit_and_reports_a_mode_drift() {
        let (source, root) = setup();
        let script = source.path().join("files/sub/b.txt");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        run(&root);
        assert_ne!(mode(root.path(), "sub/b.txt"), 0);
        assert_eq!(mode(root.path(), "a.txt"), 0);
        assert!(check(root.path()).unwrap().is_empty());
        let path = root.path().join("sub/b.txt");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(check(root.path()).unwrap(), ["modified: sub/b.txt"]);
        assert_eq!(run(&root).outcome("sub/b.txt"), Some(Outcome::Conflict));
        let force = SyncOptions {
            force: true,
            dry_run: false,
        };
        let forced = sync(root.path(), &force).unwrap();
        assert_eq!(forced.outcome("sub/b.txt"), Some(Outcome::Updated));
        assert_ne!(mode(root.path(), "sub/b.txt"), 0);
    }

    // spec: SYN-4
    #[test]
    fn a_dangling_symlink_at_a_seed_path_is_left_alone() {
        let (_source, root) = setup();
        let target = root.path().join("nowhere");
        symlink(&target, root.path().join("s.txt")).unwrap();
        assert_eq!(run(&root).outcome("s.txt"), None);
        assert!(!target.exists());
    }
}

// spec: SYN-4
#[test]
fn a_seed_on_a_managed_path_is_not_a_seed() {
    let (source, root) = setup();
    put(source.path(), "seed/a.txt", "seed");
    let report = run(&root);
    assert_eq!(report.outcome("a.txt"), Some(Outcome::Added));
    assert_eq!(
        report.entries.iter().filter(|(p, _)| p == "a.txt").count(),
        1
    );
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a1"));
}

// spec: SYN-9
#[test]
fn init_escapes_the_source_in_the_config() {
    let root = tempdir().unwrap();
    let source = tempdir().unwrap();
    let odd = source.path().join("we\"ird\\dir");
    init(root.path(), &odd.to_string_lossy(), Some("v\"1")).unwrap();
    let config = read_config(root.path()).unwrap();
    let harness = config["harness"].as_table().unwrap();
    assert_eq!(harness["source"].as_str(), Some(&*odd.to_string_lossy()));
    assert_eq!(harness["rev"].as_str(), Some("v\"1"));
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

    let done = update_in(Some(cache.path()), root.path(), Some("v1"), &opts).unwrap();
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

    let done = update_in(Some(cache.path()), root.path(), None, &opts).unwrap();
    assert_eq!(done.previous.as_deref(), Some(first.as_str()));
    assert_eq!(done.rev, second);
    assert_eq!(done.sync.outcome("a.txt"), Some(Outcome::Updated));
    assert_eq!(get(root.path(), "a.txt").as_deref(), Some("a2"));
    assert!(get(root.path(), CONFIG)
        .unwrap()
        .contains(&format!("rev = \"{second}\"")));

    // Already there: `rev` is rewritten as it was and nothing else happens.
    let before = get(root.path(), CONFIG);
    let done = update_in(Some(cache.path()), root.path(), None, &opts).unwrap();
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
    let done = update_in(Some(cache.path()), root.path(), None, &options).unwrap();
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

    assert!(update_in(Some(cache.path()), root.path(), Some("nope"), &options).is_err());

    // A revision without files/: the source cannot be read as a harness.
    git_in(repo.path(), &["rm", "-rq", "files"]);
    git_in(repo.path(), &["commit", "--quiet", "-m", "empty"]);
    assert!(update_in(Some(cache.path()), root.path(), None, &options).is_err());

    let gone = git_project(&tempdir().unwrap());
    let gone_config = get(gone.path(), CONFIG);
    assert!(update_in(Some(cache.path()), gone.path(), None, &options).is_err());

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
    let done = update_in(Some(cache.path()), root.path(), None, &options).unwrap();
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
