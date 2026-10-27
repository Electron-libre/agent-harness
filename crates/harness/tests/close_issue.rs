//! Behavior test for `harness tool close-issue`: moves tracked and untracked issues, refuses bad targets.

use std::path::Path;
use std::process::Command;

fn run(script: &Path, cwd: &Path, arg: &str) -> (bool, String) {
    let out = tool_command(script)
        .arg(arg)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

// spec: PAR-8
/// `close_issue.rs` edits the queue, so it must run on the main worktree, never on an issue's worktree.
fn refuses_a_linked_worktree(script: &Path) -> Vec<(&'static str, bool)> {
    let root = std::fs::canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("close_issue_wt_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let main = root.join("repo");
    let linked = root.join("repo-foo");
    std::fs::create_dir_all(main.join("issues")).unwrap();
    std::fs::write(main.join("issues/foo.md"), "x").unwrap();
    std::fs::write(main.join("issues/queue.md"), "- [foo](foo.md) - why\n").unwrap();
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args([
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&main)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    git(&["init", "-q", "-b", "trunk", "."]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init", "--no-verify"]);
    git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "foo",
        linked.to_str().unwrap(),
    ]);
    let (in_linked, out) = run(script, &linked, "issues/foo.md");
    let untouched = linked.join("issues/foo.md").is_file()
        && std::fs::read_to_string(linked.join("issues/queue.md"))
            .unwrap()
            .contains("[foo]");
    let (relink, _) = run(script, &linked, "--relink");
    let (by_path, by_path_out) = run(script, &main, "../repo-foo/issues/foo.md");
    let untouched_by_path = linked.join("issues/foo.md").is_file()
        && std::fs::read_to_string(linked.join("issues/queue.md"))
            .unwrap()
            .contains("[foo]");
    let (from_linked, _) = run(script, &linked, "../repo/issues/foo.md");
    let main_untouched = main.join("issues/foo.md").is_file()
        && std::fs::read_to_string(main.join("issues/queue.md"))
            .unwrap()
            .contains("[foo]");
    let (in_main, _) = run(script, &main, "issues/foo.md");
    let out = vec![
        (
            "a linked worktree is refused, naming the main one",
            !in_linked && out.contains("linked worktree") && untouched,
        ),
        ("--relink is refused there too", !relink),
        (
            "the linked worktree's issue is refused by path from the main one",
            !by_path && by_path_out.contains("linked worktree") && untouched_by_path,
        ),
        (
            "the main worktree's issue is refused by path from a linked one",
            !from_linked && main_untouched,
        ),
        (
            "the main worktree closes the issue",
            in_main && main.join("issues/done/foo.md").is_file(),
        ),
        (
            "a queue with no header is left empty",
            std::fs::read_to_string(main.join("issues/queue.md"))
                .unwrap()
                .is_empty(),
        ),
    ];
    let _ = std::fs::remove_dir_all(&root);
    out
}

// spec: PAR-14
/// The issue worktrees nest in the main one (ADR 0008): closing an issue there must not rewrite their files, ignored
/// by git (`info/exclude`) or listed as an embedded repository (a clone that never ran `start-issue`), wherever they are.
fn skips_the_nested_worktrees(script: &Path) -> Vec<(&'static str, bool)> {
    let main = std::fs::canonicalize(std::env::temp_dir())
        .unwrap()
        .join(format!("close_issue_nested_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&main);
    let nested = main.join(".claude/worktrees/bar");
    let elsewhere = main.join("docs/baz");
    std::fs::create_dir_all(main.join("issues")).unwrap();
    std::fs::create_dir_all(main.join("docs/worktrees")).unwrap();
    std::fs::write(main.join("issues/foo.md"), "x").unwrap();
    std::fs::write(main.join("issues/bar.md"), "x").unwrap();
    std::fs::write(main.join("issues/baz.md"), "x").unwrap();
    std::fs::write(main.join("NOTES.md"), "see issues/foo.md\n").unwrap();
    std::fs::write(main.join("docs/worktrees/w.md"), "see issues/foo.md\n").unwrap();
    // a tracked link into a nested worktree is not followed
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        "../.claude/worktrees/bar/NOTES.md",
        main.join("docs/link.md"),
    )
    .unwrap();
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args([
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&main)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    git(&["init", "-q", "-b", "trunk", "."]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init", "--no-verify"]);
    git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "bar",
        nested.to_str().unwrap(),
    ]);
    git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "baz",
        elsewhere.to_str().unwrap(),
    ]);
    std::fs::write(main.join("docs/draft.md"), "see issues/foo.md\n").unwrap();
    let read = |p: &Path| std::fs::read_to_string(p).unwrap();
    let kept = |wt: &Path| {
        read(&wt.join("NOTES.md")) == "see issues/foo.md\n"
            && read(&wt.join("docs/worktrees/w.md")) == "see issues/foo.md\n"
    };
    let (closed, _) = run(script, &main, "issues/foo.md");
    let nested_kept = kept(&nested);
    let elsewhere_kept = kept(&elsewhere);
    std::fs::write(main.join(".git/info/exclude"), "/.claude/worktrees/\n").unwrap();
    std::fs::write(main.join("docs/later.md"), "see issues/foo.md\n").unwrap();
    let (relinked, _) = run(script, &main, "--relink");
    std::fs::write(main.join(".git/index"), "corrupt").unwrap();
    let (relink_failed, failed_out) = run(script, &main, "--relink");
    let out = vec![
        (
            "a failing git ls-files fails, naming --relink",
            !relink_failed && failed_out.contains("--relink"),
        ),
        (
            "closing an issue rewrites the main worktree's references",
            closed && read(&main.join("NOTES.md")) == "see issues/done/foo.md\n",
        ),
        (
            "a directory named worktrees elsewhere is still scanned",
            read(&main.join("docs/worktrees/w.md")) == "see issues/done/foo.md\n",
        ),
        (
            "an untracked document of the main worktree is rewritten",
            read(&main.join("docs/draft.md")) == "see issues/done/foo.md\n",
        ),
        (
            "the files of a worktree under .claude/worktrees git does not ignore are untouched",
            nested_kept,
        ),
        (
            "the files of a worktree nested anywhere else are untouched",
            elsewhere_kept,
        ),
        (
            "--relink rewrites an untracked document",
            relinked && read(&main.join("docs/later.md")) == "see issues/done/foo.md\n",
        ),
        (
            "--relink leaves the worktree git ignores untouched",
            kept(&nested) && kept(&elsewhere),
        ),
    ];
    let _ = std::fs::remove_dir_all(&main);
    out
}

#[test]
fn close_issue() {
    let script = std::path::PathBuf::from("/tools/close_issue.rs");
    let tmp = std::env::temp_dir().join(format!("close_issue_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("issues")).unwrap();
    let git = |args: &[&str]| {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&tmp)
            .status()
            .unwrap()
            .success())
    };
    git(&["init", "-q"]);
    for f in ["tracked.md", "untracked.md"] {
        std::fs::write(tmp.join("issues").join(f), "x").unwrap();
    }
    git(&["add", "issues/tracked.md"]);
    std::fs::write(tmp.join("PLAN.md"), "see issues/tracked.md and my_issues/tracked.md, docs/issues/tracked.md, issues/tracked.md.bak2 (`issues/tracked.md`) issues/tracked_two.md issues/done/untracked.md").unwrap();

    let mut failed = false;
    let mut check = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    };
    let (ok, _) = run(&script, &tmp, "issues/tracked.md");
    let index = |path: &str| {
        let out = Command::new("git")
            .args(["ls-files", "--", path])
            .current_dir(&tmp)
            .output()
            .unwrap();
        !out.stdout.is_empty()
    };
    let staged = index("issues/done/tracked.md") && !index("issues/tracked.md");
    check(
        "tracked issue is moved with git mv",
        ok && staged
            && tmp.join("issues/done/tracked.md").is_file()
            && !tmp.join("issues/tracked.md").exists(),
    );
    let plan = std::fs::read_to_string(tmp.join("PLAN.md")).unwrap();
    check("references are pointed at done/", plan == "see issues/done/tracked.md and my_issues/tracked.md, docs/issues/tracked.md, issues/done/tracked.md.bak2 (`issues/done/tracked.md`) issues/tracked_two.md issues/done/untracked.md");
    let (ok, _) = run(&script, &tmp, "issues/untracked.md");
    let outside_index = !index("issues/done/untracked.md");
    check(
        "untracked issue is moved and left out of the index",
        ok && outside_index && tmp.join("issues/done/untracked.md").is_file(),
    );
    std::fs::write(tmp.join("issues/done/README.md"), "x").unwrap();
    std::fs::write(
        tmp.join("NOTES.md"),
        "issues/untracked.md issues/README.md issues/other.md",
    )
    .unwrap();
    std::fs::write(tmp.join("issues/other.md"), "x").unwrap();
    for dir in [".claude", "vendor/docs", "node_modules/pkg"] {
        std::fs::create_dir_all(tmp.join(dir)).unwrap();
    }
    std::fs::write(
        tmp.join(".claude/harness.toml"),
        "[project]\nskipped_dirs = [\"vendor\"]\n",
    )
    .unwrap();
    std::fs::write(tmp.join("vendor/docs/v.md"), "issues/untracked.md").unwrap();
    std::fs::write(tmp.join("node_modules/pkg/n.md"), "issues/untracked.md").unwrap();
    let (ok, _) = run(&script, &tmp, "--relink");
    let notes = std::fs::read_to_string(tmp.join("NOTES.md")).unwrap();
    check(
        "--relink repairs done issues only",
        ok && notes == "issues/done/untracked.md issues/README.md issues/other.md",
    );
    let untouched =
        |p: &str| std::fs::read_to_string(tmp.join(p)).unwrap() == "issues/untracked.md";
    check(
        "skipped_dirs and node_modules are not rewritten",
        untouched("vendor/docs/v.md") && untouched("node_modules/pkg/n.md"),
    );
    std::fs::remove_dir_all(tmp.join("node_modules")).unwrap();
    std::fs::create_dir_all(tmp.join("elsewhere")).unwrap();
    std::fs::write(tmp.join("elsewhere/x.md"), "x").unwrap();
    let (ok, out) = run(&script, &tmp, "elsewhere/x.md");
    check(
        "file outside issues/ is refused",
        !ok && out.contains("not an issue"),
    );
    let (ok, _) = run(&script, &tmp, "issues/missing.md");
    check("missing file is refused", !ok);
    std::fs::write(tmp.join("issues/tracked.md"), "y").unwrap();
    let (ok, out) = run(&script, &tmp, "issues/tracked.md");
    check(
        "existing target is not overwritten",
        !ok && out.contains("already exists"),
    );
    std::fs::write(tmp.join("issues/absolute.md"), "x").unwrap();
    git(&["add", "issues/absolute.md"]);
    let outside = std::fs::canonicalize(std::env::temp_dir()).unwrap();
    let (ok, out) = run(
        &script,
        &outside,
        tmp.join("issues/absolute.md").to_str().unwrap(),
    );
    let staged = index("issues/done/absolute.md") && !index("issues/absolute.md");
    check(
        "an absolute path from outside the repo is moved with git mv",
        ok && staged && out.contains("git mv"),
    );

    let header = "# Issue queue\n\nIntro.\n";
    std::fs::write(
        tmp.join("issues/queue.md"),
        format!("{header}\n- [a](a.md) - why\n- [b](b.md) - why\n- [c](c.md) - why\n"),
    )
    .unwrap();
    for f in ["a.md", "b.md", "c.md"] {
        std::fs::write(tmp.join("issues").join(f), "x").unwrap();
    }
    let (ok, _) = run(&script, &tmp, "issues/b.md");
    let queue = std::fs::read_to_string(tmp.join("issues/queue.md")).unwrap();
    check(
        "closing a middle entry keeps the others",
        ok && queue == format!("{header}\n- [a](a.md) - why\n- [c](c.md) - why\n"),
    );
    run(&script, &tmp, "issues/a.md");
    let (ok, _) = run(&script, &tmp, "issues/c.md");
    let queue = std::fs::read_to_string(tmp.join("issues/queue.md")).unwrap();
    check(
        "closing the last entry leaves the header ending with one newline",
        ok && queue == header,
    );

    for (name, ok) in refuses_a_linked_worktree(&script)
        .into_iter()
        .chain(skips_the_nested_worktrees(&script))
    {
        check(name, ok);
    }

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(!failed, "a check failed: see the FAIL lines");
}

/// `harness tool <name>` for the script path the checks were written against (`next_issue.rs` is `next-issue`).
fn tool_command(script: impl AsRef<std::path::Path>) -> std::process::Command {
    let stem = script
        .as_ref()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .replace('_', "-");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_harness"));
    cmd.arg("tool").arg(stem);
    cmd
}
