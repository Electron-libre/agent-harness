#!/usr/bin/env rust-script
//! Behavior test for `close_issue.rs`: moves tracked and untracked issues, refuses bad targets.
//! Usage: `just test-harness`.

use std::path::Path;
use std::process::Command;

fn run(script: &Path, cwd: &Path, arg: &str) -> (bool, String) {
    let out = Command::new("rust-script").arg(script).arg(arg).current_dir(cwd).output().expect("rust-script must be installed");
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr))
}

// spec: PAR-8
/// `close_issue.rs` edits the queue, so it must run on the main worktree, never on an issue's worktree.
fn refuses_a_linked_worktree(script: &Path) -> Vec<(&'static str, bool)> {
    let root = std::fs::canonicalize(std::env::temp_dir()).unwrap().join(format!("close_issue_wt_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let main = root.join("repo");
    let linked = root.join("repo-foo");
    std::fs::create_dir_all(main.join("issues")).unwrap();
    std::fs::write(main.join("issues/foo.md"), "x").unwrap();
    std::fs::write(main.join("issues/queue.md"), "- [foo](foo.md) - why\n").unwrap();
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args(["-c", "user.email=t@t", "-c", "user.name=t", "-c", "commit.gpgsign=false"])
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
    git(&["worktree", "add", "-q", "-b", "foo", linked.to_str().unwrap()]);
    let (in_linked, out) = run(script, &linked, "issues/foo.md");
    let untouched = linked.join("issues/foo.md").is_file() && std::fs::read_to_string(linked.join("issues/queue.md")).unwrap().contains("[foo]");
    let (relink, _) = run(script, &linked, "--relink");
    let (by_path, by_path_out) = run(script, &main, "../repo-foo/issues/foo.md");
    let untouched_by_path = linked.join("issues/foo.md").is_file() && std::fs::read_to_string(linked.join("issues/queue.md")).unwrap().contains("[foo]");
    let (in_main, _) = run(script, &main, "issues/foo.md");
    let out = vec![
        ("a linked worktree is refused, naming the main one", !in_linked && out.contains("linked worktree") && untouched),
        ("--relink is refused there too", !relink),
        ("the linked worktree's issue is refused by path from the main one", !by_path && by_path_out.contains("linked worktree") && untouched_by_path),
        ("the main worktree closes the issue", in_main && main.join("issues/done/foo.md").is_file()),
        ("a queue with no header is left empty", std::fs::read_to_string(main.join("issues/queue.md")).unwrap().is_empty()),
    ];
    let _ = std::fs::remove_dir_all(&root);
    out
}

fn main() {
    let script = std::fs::canonicalize(Path::new(file!()).parent().unwrap().join("close_issue.rs")).unwrap();
    let tmp = std::env::temp_dir().join(format!("close_issue_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("issues")).unwrap();
    let git = |args: &[&str]| assert!(Command::new("git").args(args).current_dir(&tmp).status().unwrap().success());
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
        let out = Command::new("git").args(["ls-files", "--", path]).current_dir(&tmp).output().unwrap();
        !out.stdout.is_empty()
    };
    let staged = index("issues/done/tracked.md") && !index("issues/tracked.md");
    check("tracked issue is moved with git mv", ok && staged && tmp.join("issues/done/tracked.md").is_file() && !tmp.join("issues/tracked.md").exists());
    let plan = std::fs::read_to_string(tmp.join("PLAN.md")).unwrap();
    check("references are pointed at done/", plan == "see issues/done/tracked.md and my_issues/tracked.md, docs/issues/tracked.md, issues/done/tracked.md.bak2 (`issues/done/tracked.md`) issues/tracked_two.md issues/done/untracked.md");
    let (ok, _) = run(&script, &tmp, "issues/untracked.md");
    let outside_index = !index("issues/done/untracked.md");
    check("untracked issue is moved and left out of the index", ok && outside_index && tmp.join("issues/done/untracked.md").is_file());
    std::fs::write(tmp.join("issues/done/README.md"), "x").unwrap();
    std::fs::write(tmp.join("NOTES.md"), "issues/untracked.md issues/README.md issues/other.md").unwrap();
    std::fs::write(tmp.join("issues/other.md"), "x").unwrap();
    let (ok, _) = run(&script, &tmp, "--relink");
    let notes = std::fs::read_to_string(tmp.join("NOTES.md")).unwrap();
    check("--relink repairs done issues only", ok && notes == "issues/done/untracked.md issues/README.md issues/other.md");
    std::fs::create_dir_all(tmp.join("elsewhere")).unwrap();
    std::fs::write(tmp.join("elsewhere/x.md"), "x").unwrap();
    let (ok, out) = run(&script, &tmp, "elsewhere/x.md");
    check("file outside issues/ is refused", !ok && out.contains("not an issue"));
    let (ok, _) = run(&script, &tmp, "issues/missing.md");
    check("missing file is refused", !ok);
    std::fs::write(tmp.join("issues/tracked.md"), "y").unwrap();
    let (ok, out) = run(&script, &tmp, "issues/tracked.md");
    check("existing target is not overwritten", !ok && out.contains("already exists"));
    std::fs::write(tmp.join("issues/absolute.md"), "x").unwrap();
    git(&["add", "issues/absolute.md"]);
    let outside = std::fs::canonicalize(std::env::temp_dir()).unwrap();
    let (ok, out) = run(&script, &outside, tmp.join("issues/absolute.md").to_str().unwrap());
    let staged = index("issues/done/absolute.md") && !index("issues/absolute.md");
    check("an absolute path from outside the repo is moved with git mv", ok && staged && out.contains("git mv"));

    let header = "# Issue queue\n\nIntro.\n";
    std::fs::write(tmp.join("issues/queue.md"), format!("{header}\n- [a](a.md) - why\n- [b](b.md) - why\n- [c](c.md) - why\n")).unwrap();
    for f in ["a.md", "b.md", "c.md"] {
        std::fs::write(tmp.join("issues").join(f), "x").unwrap();
    }
    let (ok, _) = run(&script, &tmp, "issues/b.md");
    let queue = std::fs::read_to_string(tmp.join("issues/queue.md")).unwrap();
    check("closing a middle entry keeps the others", ok && queue == format!("{header}\n- [a](a.md) - why\n- [c](c.md) - why\n"));
    run(&script, &tmp, "issues/a.md");
    let (ok, _) = run(&script, &tmp, "issues/c.md");
    let queue = std::fs::read_to_string(tmp.join("issues/queue.md")).unwrap();
    check("closing the last entry leaves the header ending with one newline", ok && queue == header);

    for (name, ok) in refuses_a_linked_worktree(&script) {
        check(name, ok);
    }

    let _ = std::fs::remove_dir_all(&tmp);
    if failed {
        std::process::exit(1);
    }
}
