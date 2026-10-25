#!/usr/bin/env rust-script
//! Behavior test for `require_code_review.rs`: runs `record` and `check` against a throwaway git repository.
//! Usage: `just test-harness` (or `rust-script .claude/hooks/test_require_code_review.rs`).
//!
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! ```

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Exit code and stderr of the hook in `mode` for `event`, with `project` as the project directory.
fn run(hook: &Path, mode: &str, project: &Path, event: &serde_json::Value) -> (i32, String) {
    let mut child = Command::new("rust-script")
        .arg(hook)
        .arg(mode)
        .env("CLAUDE_PROJECT_DIR", project)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("rust-script must be installed");
    child.stdin.take().unwrap().write_all(event.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn review_done(hook: &Path, project: &Path, skill: &str) {
    let event = serde_json::json!({
        "tool_name": "Skill", "cwd": project, "tool_input": {"skill": skill}, "tool_response": {"success": true}
    });
    assert_eq!(run(hook, "record", project, &event).0, 0);
}

fn commit_attempt(hook: &Path, project: &Path, command: &str) -> (i32, String) {
    let event = serde_json::json!({"tool_name": "Bash", "cwd": project, "tool_input": {"command": command}});
    run(hook, "check", project, &event)
}

fn sh(project: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(project)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}

// spec: PAR-9
/// A review recorded in one worktree must not cover another: the marker and the waiver live in the git directory
/// of each worktree.
fn git_dir_of(dir: &Path) -> std::path::PathBuf {
    let out = Command::new("git").args(["rev-parse", "--absolute-git-dir"]).current_dir(dir).output().unwrap();
    Path::new(String::from_utf8_lossy(&out.stdout).trim()).to_path_buf()
}

fn worktree_checks(hook: &Path) -> Vec<(&'static str, bool)> {
    let root = std::env::temp_dir().join(format!("reviewtest-wt-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let main = root.join("main");
    let linked = root.join("linked");
    fs::create_dir_all(&main).unwrap();
    sh(&main, &["init", "-q", "."]);
    fs::write(main.join("a.txt"), "one\n").unwrap();
    sh(&main, &["add", "-A"]);
    sh(&main, &["commit", "-qm", "init"]);
    sh(&main, &["worktree", "add", "-q", "-b", "other", linked.to_str().unwrap()]);
    let commit = "git commit -am x";
    fs::write(main.join("a.txt"), "two\n").unwrap();
    fs::write(linked.join("a.txt"), "two\n").unwrap();

    review_done(hook, &main, "code-review");
    let mut out = vec![
        ("a review in the main worktree covers it", commit_attempt(hook, &main, commit).0 == 0),
        ("the same diff in a linked worktree is still unreviewed", commit_attempt(hook, &linked, commit).0 == 2),
    ];
    review_done(hook, &linked, "code-review");
    out.push(("a review in a linked worktree covers it", commit_attempt(hook, &linked, commit).0 == 0));

    let waiver = git_dir_of(&linked).join("claude-review-waived");
    fs::write(&waiver, "").unwrap();
    out.push(("a waiver of the linked worktree lets its commit through", commit_attempt(hook, &linked, commit).0 == 0));
    out.push(("the waiver is consumed", !waiver.exists()));
    fs::write(linked.join("new.txt"), "x\n").unwrap();
    let main_waiver = git_dir_of(&main).join("claude-review-waived");
    fs::write(&main_waiver, "").unwrap();
    out.push(("the waiver of the main worktree does not cover the linked one", commit_attempt(hook, &linked, commit).0 == 2));
    out.push(("the linked worktree leaves the main waiver alone", main_waiver.exists()));
    let _ = fs::remove_dir_all(&root);
    out
}

fn main() {
    let here = fs::canonicalize(Path::new(file!()).parent().unwrap_or(Path::new("."))).unwrap();
    let hook = here.join("require_code_review.rs");
    let project = std::env::temp_dir().join(format!("reviewtest-{}", std::process::id()));
    let _ = fs::remove_dir_all(&project);
    fs::create_dir_all(&project).unwrap();
    sh(&project, &["init", "-q", "."]);
    fs::write(project.join("a.txt"), "one\n").unwrap();
    sh(&project, &["add", "-A"]);
    sh(&project, &["commit", "-qm", "init"]);

    let mut failed = false;
    let mut check = |name: &str, ok: bool| {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    };
    let commit = "git add -A && git commit -m \"feat: x\"";

    check("lets through a commit with nothing changed", commit_attempt(&hook, &project, commit).0 == 0);

    fs::write(project.join("a.txt"), "two\n").unwrap();
    let (code, stderr) = commit_attempt(&hook, &project, commit);
    check("refuses a commit without any review", code == 2 && stderr.contains("code-review"));
    check("lets through a command that is not a commit", commit_attempt(&hook, &project, "git status").0 == 0);
    check("lets through git log mentioning commit", commit_attempt(&hook, &project, "git log --grep commit").0 == 0);

    review_done(&hook, &project, "simplify");
    check("ignores the end of another skill", commit_attempt(&hook, &project, commit).0 == 2);

    review_done(&hook, &project, "code-review");
    check("lets through a commit of the reviewed diff", commit_attempt(&hook, &project, commit).0 == 0);

    fs::write(project.join("a.txt"), "two, fixed\n").unwrap();
    check("accepts a fix made after the review in a reviewed file", commit_attempt(&hook, &project, commit).0 == 0);
    fs::write(project.join("a.txt"), "two\n").unwrap();

    fs::write(project.join("new.txt"), "x\n").unwrap();
    check("refuses a file the review never saw", commit_attempt(&hook, &project, commit).0 == 2);

    fs::remove_file(project.join("new.txt")).unwrap();
    check("accepts the reviewed diff again", commit_attempt(&hook, &project, commit).0 == 0);

    sh(&project, &["commit", "-aqm", "feat: reviewed"]);
    fs::write(project.join("a.txt"), "three\n").unwrap();
    check("refuses a review made before the last commit", commit_attempt(&hook, &project, commit).0 == 2);

    fs::create_dir_all(project.join("deep")).unwrap();
    let other = project.join("sub");
    fs::create_dir_all(&other).unwrap();
    sh(&other, &["init", "-q", "."]);
    fs::write(other.join("b.txt"), "x\n").unwrap();
    let relative = serde_json::json!({"tool_name": "Bash", "cwd": project.join("deep"), "tool_input": {"command": "git -C ../sub commit -am x"}});
    check("resolves -C from the shell directory", run(&hook, "check", &project, &relative).0 == 2);
    let from_deep = serde_json::json!({"tool_name": "Bash", "cwd": project.join("deep"), "tool_input": {"command": "git commit -am x"}});
    check("sees the whole work tree from a subdirectory", run(&hook, "check", &project, &from_deep).0 == 2);
    check("sees a commit ended by a semicolon", commit_attempt(&hook, &project, "git add .;git commit;").0 == 2);
    check("looks at the repository named by -C", commit_attempt(&hook, &project, "git -C sub commit -am x").0 == 2);
    check("sees a commit in brackets", commit_attempt(&hook, &project, "(git commit -m x)").0 == 2);
    for option in ["--git-dir", "--work-tree", "--namespace", "--exec-path", "--super-prefix"] {
        let spaced = format!("git {option} /x commit -m x");
        check(&format!("sees a commit after {option} given as two words"), commit_attempt(&hook, &project, &spaced).0 == 2);
    }
    check("sees a commit after a quoted option value", commit_attempt(&hook, &project, "git --namespace 'a b' commit -m x").0 == 2);
    check("sees a commit after --namespace=value", commit_attempt(&hook, &project, "git --namespace=n commit -m x").0 == 2);
    check("sees a commit after -c with a quoted value", commit_attempt(&hook, &project, "git -c 'user.name=A B' commit -m x").0 == 2);
    check("looks at the repository named by a quoted -C", commit_attempt(&hook, &project, "git -C 'sub' commit -am x").0 == 2);
    check("ignores a path named commit after an option value", commit_attempt(&hook, &project, "git --git-dir /x log -- commit").0 == 0);
    check("ignores a path named commit", commit_attempt(&hook, &project, "git log -- commit").0 == 0);

    review_done(&hook, &project, "code-review");
    check("lets through the reviewed diff of the next change", commit_attempt(&hook, &project, commit).0 == 0);
    fs::write(project.join("é.txt"), "x\n").unwrap();
    check("refuses a new non-ASCII file", commit_attempt(&hook, &project, commit).0 == 2);
    fs::remove_file(project.join("é.txt")).unwrap();

    let failed_review = serde_json::json!({
        "tool_name": "Skill", "cwd": project, "tool_input": {"skill": "code-review"}, "tool_response": {"success": false}
    });
    fs::write(project.join("f.txt"), "z\n").unwrap();
    run(&hook, "record", &project, &failed_review);
    check("does not record a failed review", commit_attempt(&hook, &project, commit).0 == 2);

    let esc = project.join("esc");
    fs::create_dir_all(esc.join("issues")).unwrap();
    sh(&esc, &["init", "-q", "."]);
    fs::write(esc.join("issues/i.md"), "one\n").unwrap();
    fs::write(esc.join("code.txt"), "one\n").unwrap();
    sh(&esc, &["add", "-A"]);
    sh(&esc, &["commit", "-qm", "init"]);
    let esc_commit = "git -C esc commit -am x";
    fs::write(esc.join("issues/i.md"), "two\n").unwrap();
    check("lets through a commit that only touches issues", commit_attempt(&hook, &project, esc_commit).0 == 0);
    fs::write(esc.join("issues/j.md"), "new\n").unwrap();
    check("lets through a new issue file", commit_attempt(&hook, &project, esc_commit).0 == 0);
    sh(&esc, &["mv", "code.txt", "issues/code.txt"]);
    check("refuses a code file renamed into issues", commit_attempt(&hook, &project, esc_commit).0 == 2);
    sh(&esc, &["mv", "issues/code.txt", "code.txt"]);
    fs::write(esc.join("code.txt"), "two\n").unwrap();
    check("refuses issues mixed with code", commit_attempt(&hook, &project, esc_commit).0 == 2);
    fs::write(esc.join("docs.md"), "doc\n").unwrap();
    fs::write(esc.join("code.txt"), "one\n").unwrap();
    check("refuses documentation", commit_attempt(&hook, &project, esc_commit).0 == 2);
    let waiver = esc.join(".git/claude-review-waived");
    check("refuses a command that creates the waiver", commit_attempt(&hook, &project, "touch esc/.git/claude-review-waived").0 == 2);
    fs::write(&waiver, "").unwrap();
    check("lets through a commit the user waived", commit_attempt(&hook, &project, esc_commit).0 == 0);
    check("consumes the waiver", !waiver.exists() && commit_attempt(&hook, &project, esc_commit).0 == 2);

    let fresh = project.join("fresh");
    fs::create_dir_all(&fresh).unwrap();
    sh(&fresh, &["init", "-q", "."]);
    fs::write(fresh.join("c.txt"), "x\n").unwrap();
    check("refuses the first commit of a repository", commit_attempt(&hook, &project, "git -C fresh commit -m x").0 == 2);
    review_done(&hook, &fresh, "code-review");
    check("lets through the reviewed first commit", commit_attempt(&hook, &project, "git -C fresh commit -m x").0 == 0);

    for (name, ok) in worktree_checks(&hook) {
        check(name, ok);
    }

    let _ = fs::remove_dir_all(&project);
    std::process::exit(i32::from(failed));
}
