#!/usr/bin/env rust-script
//! Behavior test for `issue_worktree.rs` (`start` and `finish`) against a throwaway git repository.
//! Usage: `just test-harness`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
        .status
        .success()
}

fn git_ok(dir: &Path, args: &[&str]) {
    assert!(git(dir, args), "git {args:?} failed");
}

fn branch_exists(repo: &Path, name: &str) -> bool {
    git(repo, &["rev-parse", "--verify", "-q", &format!("refs/heads/{name}")])
}

struct Ctx {
    root: PathBuf,
    repo: PathBuf,
    script: PathBuf,
}

type Checks = Vec<(&'static str, bool)>;

impl Ctx {
    fn run(&self, cwd: &Path, args: &[&str]) -> (bool, String) {
        let out = Command::new("rust-script").arg(&self.script).args(args).current_dir(cwd).output().expect("rust-script must be installed");
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr))
    }

    fn wt(&self, stem: &str) -> PathBuf {
        self.root.join(format!("repo-{stem}"))
    }
}

// spec: PAR-1
fn start_creates_the_worktree(c: &Ctx) -> Checks {
    let (ok, out) = c.run(&c.repo, &["start", "issues/x.md"]);
    let created = ok && c.wt("x").join("issues/x.md").is_file() && branch_exists(&c.repo, "x");
    let (linked_ok, _) = c.run(&c.wt("x"), &["start", "issues/y.md"]);
    vec![
        ("start creates the sibling worktree on a branch named after the stem", created),
        ("start prints the worktree path", out.contains(c.wt("x").to_str().unwrap())),
        ("start works from a linked worktree and names the sibling of the main one", linked_ok && c.wt("y").is_dir()),
    ]
}

// spec: PAR-2
fn start_refuses_and_changes_nothing(c: &Ctx) -> Checks {
    let before = std::fs::read_dir(&c.root).unwrap().count();
    let (taken, _) = c.run(&c.repo, &["start", "issues/x.md"]);
    let (missing, _) = c.run(&c.repo, &["start", "issues/missing.md"]);
    let (uncommitted, _) = c.run(&c.repo, &["start", "issues/new.md"]);
    std::fs::create_dir_all(c.wt("z")).unwrap();
    let (occupied, _) = c.run(&c.repo, &["start", "issues/z.md"]);
    std::fs::remove_dir_all(c.wt("z")).unwrap();
    vec![
        ("start refuses an issue whose branch exists", !taken),
        ("start refuses a missing issue file", !missing && !branch_exists(&c.repo, "missing")),
        ("start refuses an issue file the main branch does not hold", !uncommitted && !branch_exists(&c.repo, "new")),
        ("start refuses an existing directory", !occupied && !branch_exists(&c.repo, "z")),
        ("a refusal changes nothing", std::fs::read_dir(&c.root).unwrap().count() == before),
    ]
}

// spec: PAR-6
fn finish_needs_a_merged_branch(c: &Ctx) -> Checks {
    std::fs::write(c.wt("x").join("work.txt"), "work\n").unwrap();
    git_ok(&c.wt("x"), &["add", "-A"]);
    git_ok(&c.wt("x"), &["commit", "-qm", "work", "--no-verify"]);
    let (unmerged, _) = c.run(&c.repo, &["finish", "x"]);
    let kept = c.wt("x").is_dir() && branch_exists(&c.repo, "x");
    git_ok(&c.repo, &["merge", "-q", "--ff-only", "x"]);
    let (merged, _) = c.run(&c.repo, &["finish", "x"]);
    vec![
        ("finish refuses an unmerged branch and keeps both", !unmerged && kept),
        ("finish removes the worktree and the branch once merged", merged && !c.wt("x").exists() && !branch_exists(&c.repo, "x")),
    ]
}

// spec: PAR-7
fn finish_removes_only_a_clean_sibling(c: &Ctx) -> Checks {
    std::fs::write(c.wt("y").join("dirty.txt"), "wip\n").unwrap();
    let (dirty, _) = c.run(&c.repo, &["finish", "y"]);
    let elsewhere = c.root.join("elsewhere");
    git_ok(&c.repo, &["worktree", "add", "-q", "-b", "w", elsewhere.to_str().unwrap()]);
    let (stranger, _) = c.run(&c.repo, &["finish", "w"]);
    vec![
        ("finish refuses a worktree with uncommitted changes", !dirty && c.wt("y").is_dir() && branch_exists(&c.repo, "y")),
        ("finish does not remove a worktree that is not the sibling of the stem", !stranger && elsewhere.is_dir() && branch_exists(&c.repo, "w")),
    ]
}

fn main() {
    let script = std::fs::canonicalize(Path::new(file!()).parent().unwrap()).unwrap().join("issue_worktree.rs");
    let root = std::fs::canonicalize(std::env::temp_dir()).unwrap().join(format!("issue_worktree_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(repo.join("issues")).unwrap();
    for s in ["x", "y", "z", "w"] {
        std::fs::write(repo.join(format!("issues/{s}.md")), format!("# {s}\n")).unwrap();
    }
    git_ok(&repo, &["init", "-q", "-b", "trunk", "."]);
    git_ok(&repo, &["add", "-A"]);
    git_ok(&repo, &["commit", "-qm", "init", "--no-verify"]);
    std::fs::write(repo.join("issues/new.md"), "# new\n").unwrap();

    let c = Ctx { root: root.clone(), repo, script };
    let mut failed = false;
    for checks in [start_creates_the_worktree(&c), start_refuses_and_changes_nothing(&c), finish_needs_a_merged_branch(&c), finish_removes_only_a_clean_sibling(&c)] {
        for (name, ok) in checks {
            println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
            failed |= !ok;
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    if failed {
        std::process::exit(1);
    }
}
