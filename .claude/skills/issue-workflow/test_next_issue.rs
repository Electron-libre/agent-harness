#!/usr/bin/env rust-script
//! Behavior test for `next_issue.rs` and the queue handling of `close_issue.rs`.
//! Usage: `just test-harness`.

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "git {args:?} failed");
}

// spec: PAR-3, PAR-4, PAR-5
/// An issue is taken when a branch named after its stem exists or a worktree has it checked out; `next_issue.rs`
/// skips it, names it, and behaves as before when nothing is taken.
fn taken_checks(script: &Path) -> Vec<(&'static str, bool)> {
    let root = std::env::temp_dir().join(format!("next_issue_taken_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(repo.join("issues")).unwrap();
    for s in ["a", "b", "c", "d"] {
        std::fs::write(repo.join(format!("issues/{s}.md")), format!("# Title {s}\n")).unwrap();
    }
    std::fs::write(repo.join("issues/queue.md"), "# Queue\n\n- [a](a.md) - one\n- [b](b.md) - two\n- [c](c.md) - three\n- [d](d.md) - four\n").unwrap();
    git(&repo, &["init", "-q", "-b", "trunk", "."]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "init", "--no-verify"]);
    let run = |cwd: &Path| {
        let out = Command::new("rust-script").arg(script).arg("issues").current_dir(cwd).output().unwrap();
        (out.status.code(), String::from_utf8_lossy(&out.stdout).into_owned())
    };

    let (code, free) = run(&repo);
    let mut out = vec![("nothing taken keeps the output", code == Some(0) && free.contains("Next: issues/a.md") && !free.contains("aken"))];
    git(&repo, &["branch", "a"]);
    let (_, branch) = run(&repo);
    out.push((
        "a branch named after the stem takes the issue, the others keep their queue position",
        branch.contains("Next: issues/b.md") && branch.contains("Taken: a\n") && branch.contains("  3. c") && branch.contains("  4. d"),
    ));
    let linked = root.join("repo-b");
    git(&repo, &["worktree", "add", "-q", "-b", "b", linked.to_str().unwrap()]);
    let (_, mid) = run(&repo);
    out.push(("a worktree path is named", mid.contains("Taken: b (worktree ") && mid.contains("repo-b)")));
    git(&repo, &["branch", "c"]);
    git(&repo, &["branch", "d"]);
    let (code, all) = run(&repo);
    out.push(("all taken is said", code == Some(0) && all.contains("Every issue") && !all.contains("Next:")));
    let (_, elsewhere) = {
        let o = Command::new("rust-script").arg(script).arg(repo.join("issues")).current_dir(&root).output().unwrap();
        (o.status.code(), String::from_utf8_lossy(&o.stdout).into_owned())
    };
    out.push(("git is queried in the repository of the issues directory", elsewhere.contains("Every issue")));
    git(&repo, &["branch", "-D", "d"]);
    git(&repo, &["branch", "-m", "trunk", "d"]);
    let (_, trunk) = run(&repo);
    out.push(("the branch of the main worktree is not an issue", trunk.contains("Taken: a") && !trunk.contains("Taken: d")));
    git(&repo, &["branch", "-m", "d", "trunk"]);
    git(&repo, &["branch", "d"]);
    let (_, from_linked) = run(&linked);
    out.push(("the mark is seen from another worktree", from_linked.contains("repo-b") && !from_linked.contains("Next:")));
    let plain = root.join("plain");
    std::fs::create_dir_all(plain.join("issues")).unwrap();
    std::fs::copy(repo.join("issues/a.md"), plain.join("issues/a.md")).unwrap();
    std::fs::write(plain.join("issues/queue.md"), "- [a](a.md) - one\n").unwrap();
    let (code, outside) = run(&plain);
    out.push(("outside a git repository nothing is taken", code == Some(0) && outside.contains("Next: issues/a.md")));
    let _ = std::fs::remove_dir_all(&root);
    out
}

fn main() {
    let dir = std::fs::canonicalize(Path::new(file!()).parent().unwrap()).unwrap();
    let tmp = std::env::temp_dir().join(format!("next_issue_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("issues")).unwrap();
    let w = |f: &str, t: &str| std::fs::write(tmp.join("issues").join(f), t).unwrap();
    w("a.md", "# Title A\n");
    w("b.md", "# Title B\n");
    w("c.md", "# Title C\n");
    w("queue.md", "# Issue queue\n\nProse.\n\n- [b](b.md) - first\n- [a](a.md) - second\n- [ghost](ghost.md) - gone\n");
    let run = |script: &str, arg: &str| {
        let out = Command::new("rust-script").arg(dir.join(script)).arg(arg).current_dir(&tmp).output().expect("rust-script must be installed");
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr))
    };
    let mut failed = false;
    let mut check = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    };
    let (ok, out) = run("next_issue.rs", "issues");
    check("first line is the next issue, with its title", out.contains("Next: issues/b.md") && out.contains("Title B"));
    check("line without a file is reported", !ok && out.contains("queue line without a file: ghost"));
    check("issue absent from the queue is reported", out.contains("missing from the queue: c"));
    let (_, out) = run("close_issue.rs", "issues/b.md");
    let queue = std::fs::read_to_string(tmp.join("issues/queue.md")).unwrap();
    check("closing removes the line from the queue", out.contains("removed b") && !queue.contains("[b](b.md) - first") && queue.contains("[a](a.md) - second"));
    let (_, out) = run("next_issue.rs", "issues");
    check("next moves on after closing", out.contains("Next: issues/a.md"));

    for (name, ok) in taken_checks(&dir.join("next_issue.rs")) {
        check(name, ok);
    }

    let _ = std::fs::remove_dir_all(&tmp);
    if failed {
        std::process::exit(1);
    }
}
