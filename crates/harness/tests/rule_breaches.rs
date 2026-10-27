//! Behavior test for `rule_breaches.rs` on fixture transcripts and repositories: the findings of each rule, `breach`
//! apart from `blocked`, their locations, the summary per rule and revision, no message text.
//! Usage: `just test-harness`.

use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

const COLUMNS: &str = "session\tproject\trevision\trule\tkind\tlocation\tdetail";
const SUMMARY: &str = "rule\trevision\tapplied\tbreached\tbreaches\tblocked";

fn sh(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success());
    assert!(ok, "git {args:?} failed in {}", dir.display());
}

/// Commits `files` (path, content) in `repo` at `date`, with `message`; returns the abbreviated hash.
fn commit_at(repo: &Path, date: &str, message: &str, files: &[(&str, &str)]) -> String {
    for (path, body) in files {
        let file = repo.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, body).unwrap();
    }
    sh(repo, &["add", "-A", "--force"]);
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            message,
        ])
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "commit failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let head = Command::new("git")
        .args(["log", "-1", "--format=%h"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&head.stdout).trim().to_string()
}

fn new_repo(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    sh(dir, &["init", "-q", "-b", "main"]);
    dir.to_path_buf()
}

/// A transcript under construction: each push returns the 1-based line number.
struct Transcript {
    cwd: String,
    ts: String,
    lines: Vec<String>,
}

impl Transcript {
    fn new(cwd: &Path, ts: &str) -> Self {
        Transcript {
            cwd: cwd.display().to_string(),
            ts: ts.to_string(),
            lines: Vec::new(),
        }
    }

    fn push(&mut self, mut line: serde_json::Value) -> usize {
        line["cwd"] = json!(self.cwd);
        line["timestamp"] = json!(self.ts);
        line["sessionId"] = json!("x");
        self.lines.push(line.to_string());
        self.lines.len()
    }

    fn typed(&mut self, text: &str) -> usize {
        self.push(json!({"type": "user", "gitBranch": "main", "message": {"role": "user", "content": text}}))
    }

    fn call(&mut self, id: &str, name: &str, input: serde_json::Value) -> usize {
        self.push(json!({"type": "assistant", "message": {"id": format!("m{id}"), "content": [{"type": "tool_use", "id": id, "name": name, "input": input}]}}))
    }

    fn text(&mut self, id: &str, text: &str) -> usize {
        self.push(json!({"type": "assistant", "message": {"id": id, "content": [{"type": "text", "text": text}]}}))
    }

    fn result(&mut self, id: &str, error: bool, text: &str) -> usize {
        self.push(json!({"type": "user", "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": id, "is_error": error, "content": text}]}}))
    }

    /// A call and its result; returns the line of the call.
    fn run(
        &mut self,
        id: &str,
        name: &str,
        input: serde_json::Value,
        error: Option<&str>,
    ) -> usize {
        let at = self.call(id, name, input);
        self.result(id, error.is_some(), error.unwrap_or("SECRET output"));
        at
    }

    fn write(&self, path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            path,
            self.lines
                .iter()
                .map(|l| format!("{l}\n"))
                .collect::<String>(),
        )
        .unwrap();
    }
}

fn bash(command: &str) -> serde_json::Value {
    json!({"command": command})
}

fn edit(path: &Path) -> serde_json::Value {
    json!({"file_path": path, "old_string": "SECRET old", "new_string": "SECRET new"})
}

fn run(dir: &Path, projects: &Path, summary: bool) -> (bool, String) {
    let mut cmd = tool_command(dir.join("rule_breaches.rs"));
    cmd.arg(projects);
    if summary {
        cmd.arg("--summary");
    }
    let out = cmd.output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// Builds the fixture; returns the projects directory and the expected finding rows (without header).
fn fixture(tmp: &Path) -> (PathBuf, Vec<String>) {
    let repo = new_repo(&tmp.join("proj"));
    let toml = |rev: &str| {
        format!("[harness]\nsource = \"https://example.org/harness\"\nrev = \"{rev}\"\n")
    };
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[
            (".claude/harness.toml", &toml("r1")),
            (
                ".claude/harness.lock",
                "abc+x  .claude/hooks/x.rs\ndef  AGENTS.md\n",
            ),
        ],
    );
    commit_at(
        &repo,
        "2026-03-01T00:00:00Z",
        "chore: bump",
        &[(".claude/harness.toml", &toml("r2"))],
    );
    let bare = new_repo(&tmp.join("bare"));
    commit_at(&bare, "2026-01-01T00:00:00Z", "init", &[("README", "x")]);
    let projects = tmp.join("projects");
    let proj = projects.join("-proj");
    let r = |p: &str| repo.join(p);

    let mut a = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
    a.typed("please SECRET");
    let no_verify = a.run("t1", "Bash", bash("git commit --no-verify -m SECRET"), Some("PreToolUse:Bash hook error: [x/require_code_review.rs check]: Refused: --no-verify skips the gate."));
    let refused1 = a.call(
        "t2",
        "Bash",
        bash("git commit -m \"SECRET mentions --no-verify\""),
    );
    a.result(
        "t2",
        true,
        "Refused: the last code review predates the last commit.",
    );
    a.run(
        "t3",
        "Skill",
        json!({"skill": "code-review", "args": "low"}),
        None,
    );
    a.run(
        "t4",
        "Bash",
        bash("git commit -q -F - <<'EOF'\nfeat: SECRET\n\n--no-verify\nEOF\n"),
        None,
    );
    a.call("t5", "Bash", bash("git commit -m x"));
    let refused2 = a.result("t5", true, "Refused: the diff holds files the last code review never saw. Or ask the user to waive the review.");
    let bypass = a.run("t6", "Bash", bash("git add a && git commit -m y"), None);
    let waive = a.run(
        "t7",
        "Bash",
        bash("just waive-review"),
        Some("Permission to use Bash has been denied."),
    );
    let waiver_file = a.run(
        "t8",
        "Write",
        json!({"file_path": r(".git/claude-review-waived"), "content": ""}),
        None,
    );
    a.call("t9", "Bash", bash("touch somewhere"));
    let waive_refused = a.result(
        "t9",
        true,
        "PreToolUse:Bash hook error: [x]: Refused: only the user waives the code review.",
    );
    let managed = a.run("t10", "Edit", edit(&r(".claude/hooks/x.rs")), None);
    a.run("t11", "Edit", edit(&r("files/.claude/hooks/x.rs")), None);
    a.run(
        "t12",
        "Write",
        json!({"file_path": r("issues/a.md"), "content": "SECRET\n# Tracking\n\nPlan: proposed\n"}),
        None,
    );
    a.run("t13", "Edit", edit(&r("specs/a.md")), None);
    a.run("t14", "Edit", edit(&r("docs/adr/0001-a.md")), None);
    let early = a.run("t15", "Edit", edit(&r("src/main.rs")), None);
    a.run("t16", "Edit", edit(Path::new("/elsewhere/memory.md")), None);
    a.typed("ok");
    a.run("t17", "Edit", edit(&r("src/lib.rs")), None);
    a.write(&proj.join("a.jsonl"));
    let mut sub = Transcript::new(&repo, "2026-02-01T10:30:00.000Z");
    let pushed = sub.run("s1", "Bash", bash("git push --no-verify"), None);
    sub.write(&proj.join("a/subagents/agent-x.jsonl"));

    let mut b = Transcript::new(&repo, "2026-02-02T10:00:00.000Z");
    b.typed("hello");
    b.run("t1", "Edit", edit(&r("src/x.rs")), None);
    b.run(
        "t2",
        "Write",
        json!({"file_path": r("tests/t.rs"), "content": "SECRET \"Plan: proposed\""}),
        None,
    );
    b.run("t3", "Edit", edit(&r("src/y.rs")), None);
    b.run("t4", "Bash", bash("python3 edit.py"), Some("PreToolUse:Bash hook error: [x]: Refused: the diff changed since the last code review."));
    b.run("t5", "Bash", bash("git commit -m z"), None);
    b.write(&proj.join("b.jsonl"));

    let mut c = Transcript::new(&bare, "2026-02-02T10:00:00.000Z");
    c.run("t1", "Bash", bash("git commit --no-verify -m x"), None);
    c.write(&projects.join("-bare/c.jsonl"));

    let mut d = Transcript::new(&repo, "2025-12-01T10:00:00.000Z");
    d.run("t1", "Bash", bash("git commit --no-verify -m x"), None);
    d.write(&proj.join("d.jsonl"));

    let mut g = Transcript::new(&repo, "2026-02-03T10:00:00.000Z");
    g.call("t1", "Bash", bash("git commit -m x"));
    let refused3 = g.result(
        "t1",
        true,
        "Refused: no code review has run in this project.",
    );
    g.typed("<command-name>/code-review</command-name>\n<command-args>low</command-args>");
    g.run("t2", "Bash", bash("git commit -m x"), None);
    g.write(&proj.join("g.jsonl"));

    let linked = tmp.join("proj-foo");
    sh(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "foo",
            linked.to_str().unwrap(),
        ],
    );
    let mut h = Transcript::new(&repo, "2026-02-04T10:00:00.000Z");
    h.typed("start the issue");
    h.cwd = linked.display().to_string();
    let switched = h.run("t1", "Edit", edit(&linked.join(".claude/hooks/x.rs")), None);
    let quoted = h.run(
        "t2",
        "Bash",
        bash("echo \"a << b\"\ngit commit --no-verify -m x"),
        None,
    );
    h.write(&proj.join("h.jsonl"));

    let mut e = Transcript::new(&repo, "2026-03-02T10:00:00.000Z");
    let late = e.run("t1", "Bash", bash("git -C x commit --no-verify"), None);
    e.write(&proj.join("e.jsonl"));

    let row = |s: &str, rev: &str, rule: &str, kind: &str, loc: String, detail: &str| {
        format!("{s}\tproj\t{rev}\t{rule}\t{kind}\t{loc}\t{detail}")
    };
    let at = |n: usize| format!("a.jsonl:{n}");
    let expected = vec![
        row("a", "r1", "no-verify", "blocked", at(no_verify), ""),
        row(
            "a",
            "r1",
            "review-before-commit",
            "blocked",
            at(refused1 + 1),
            "",
        ),
        row(
            "a",
            "r1",
            "review-before-commit",
            "blocked",
            at(refused2),
            "",
        ),
        row("a", "r1", "review-before-commit", "breach", at(bypass), ""),
        row("a", "r1", "review-waiver", "blocked", at(waive), ""),
        row("a", "r1", "review-waiver", "breach", at(waiver_file), ""),
        row("a", "r1", "review-waiver", "blocked", at(waive_refused), ""),
        row(
            "a",
            "r1",
            "managed-copy",
            "breach",
            at(managed),
            ".claude/hooks/x.rs",
        ),
        row(
            "a",
            "r1",
            "validation-stop",
            "breach",
            at(early),
            "src/main.rs",
        ),
        row(
            "a",
            "r1",
            "no-verify",
            "breach",
            format!("subagents/agent-x.jsonl:{pushed}"),
            "",
        ),
        row(
            "e",
            "r2",
            "no-verify",
            "breach",
            format!("e.jsonl:{late}"),
            "",
        ),
        row(
            "g",
            "r1",
            "review-before-commit",
            "blocked",
            format!("g.jsonl:{refused3}"),
            "",
        ),
        row(
            "h",
            "r1",
            "managed-copy",
            "breach",
            format!("h.jsonl:{switched}"),
            ".claude/hooks/x.rs",
        ),
        row(
            "h",
            "r1",
            "no-verify",
            "breach",
            format!("h.jsonl:{quoted}"),
            "",
        ),
    ];
    (projects, expected)
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

// spec: BRE-1, BRE-2, BRE-4, BRE-5, BRE-6, BRE-7, BRE-8, BRE-11
fn finding_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let (projects, expected) = fixture(tmp);
    let (ok, out) = run(dir, &projects, false);
    let mut lines = out.lines();
    let header = lines.next().unwrap_or_default().to_string();
    let rows: Vec<String> = lines.map(str::to_string).collect();
    let mut checks = vec![
        ("the script succeeds".to_string(), ok),
        (
            "the header names the columns".to_string(),
            header == COLUMNS,
        ),
        (
            "no text of a message, a tool input or a tool result is printed".to_string(),
            !out.contains("SECRET"),
        ),
        (
            "no finding for a project without harness, nor before its harness".to_string(),
            !out.contains("\tbare\t") && !out.contains("d.jsonl"),
        ),
        (
            "no finding beyond the expected ones".to_string(),
            sorted(rows.clone()) == sorted(expected.clone()),
        ),
    ];
    for want in expected {
        checks.push((format!("finds {want}"), rows.contains(&want)));
    }
    if !checks.iter().all(|(_, ok)| *ok) {
        println!("{out}");
    }
    checks
}

// spec: BRE-3
fn summary_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let (projects, _) = fixture(&tmp.join("summary"));
    let (ok, out) = run(dir, &projects, true);
    let expected = [
        SUMMARY,
        "managed-copy\tr1\t3\t2\t2\t0",
        "no-verify\tr1\t4\t2\t2\t1",
        "no-verify\tr2\t1\t1\t1\t0",
        "review-before-commit\tr1\t4\t1\t1\t3",
        "review-before-commit\tr2\t1\t0\t0\t0",
        "review-waiver\tr1\t4\t1\t1\t2",
        "review-waiver\tr2\t1\t0\t0\t0",
        "validation-stop\tr1\t1\t1\t1\t0",
    ];
    let lines: Vec<&str> = out.lines().collect();
    if lines != expected {
        println!("{out}");
    }
    vec![(
        "the summary counts per rule and revision, sorted".to_string(),
        ok && lines == expected,
    )]
}

// spec: BRE-9
fn commit_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("commits");
    let repo = new_repo(&tmp.join("cm"));
    let toml = "[harness]\nsource = \"https://example.org/harness\"\nrev = \"r1\"\n\n[project]\ncommit_scopes = [\"sync\"]\n";
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/harness.toml", toml)],
    );
    commit_at(&repo, "2026-02-01T09:00:00Z", "Not mine.", &[("a", "1")]);
    commit_at(
        &repo,
        "2026-02-01T10:15:00Z",
        "feat(sync): add x",
        &[("a", "2")],
    );
    let long = commit_at(
        &repo,
        "2026-02-01T10:25:00Z",
        &format!("feat: {}", "a".repeat(70)),
        &[("a", "3")],
    );
    let scope = commit_at(&repo, "2026-02-01T10:45:00Z", "fix(nope): y", &[("a", "4")]);
    let trailer = commit_at(
        &repo,
        "2026-02-01T10:50:00Z",
        "docs: z\n\nCo-Authored-By: Claude <noreply@anthropic.com>",
        &[("a", "5")],
    );
    let capital = commit_at(&repo, "2026-02-01T11:30:00Z", "Feat: Big.", &[("a", "6")]);
    sh(&repo, &["checkout", "-q", "-b", "other", "HEAD~3"]);
    let generated = commit_at(
        &repo,
        "2026-02-01T10:20:00Z",
        "chore(harness): w\n\nGenerated with [Claude Code]",
        &[("b", "1")],
    );
    let with_cli = toml.replace("[\"sync\"]", "[\"sync\", \"cli\"]");
    commit_at(
        &repo,
        "2026-02-01T10:21:00Z",
        "chore(sync): add the cli scope",
        &[(".claude/harness.toml", &with_cli)],
    );
    commit_at(
        &repo,
        "2026-02-01T10:22:00Z",
        "feat(cli): add x",
        &[("b", "2")],
    );
    sh(&repo, &["checkout", "-q", "main"]);
    std::fs::write(repo.join("a"), "stashed").unwrap();
    let stash = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "stash", "-q"])
        .env("GIT_AUTHOR_DATE", "2026-02-01T10:40:00Z")
        .env("GIT_COMMITTER_DATE", "2026-02-01T10:40:00Z")
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(stash.status.success(), "stash failed");
    std::fs::write(repo.join("c"), "1").unwrap();
    sh(&repo, &["add", "c"]);
    let rebased = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            "Mine, rebased.",
        ])
        .env("GIT_AUTHOR_DATE", "2026-01-15T10:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-02-01T10:41:00Z")
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(rebased.status.success(), "rebased commit failed");
    let merge = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "Merge branch other.",
            "other",
        ])
        .env("GIT_AUTHOR_DATE", "2026-02-01T11:40:00Z")
        .env("GIT_COMMITTER_DATE", "2026-02-01T11:40:00Z")
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(merge.status.success(), "merge failed");
    let projects = tmp.join("projects");
    for (id, start, end) in [
        ("s1", "2026-02-01T10:00:00.000Z", "2026-02-01T11:00:00.000Z"),
        ("s2", "2026-02-01T10:30:00.000Z", "2026-02-01T12:00:00.000Z"),
    ] {
        let mut t = Transcript::new(&repo, start);
        t.typed("SECRET start");
        t.ts = end.to_string();
        t.typed("ok");
        t.write(&projects.join("-cm").join(format!("{id}.jsonl")));
    }
    let row = |s: &str, rule: &str, hash: &str| format!("{s}\tcm\tr1\t{rule}\tbreach\t\t{hash}");
    let expected = vec![
        row("s1", "commit-format", &long),
        row("s1", "commit-attribution", &generated),
        row("s2", "commit-format", &scope),
        row("s2", "commit-attribution", &trailer),
        row("s2", "commit-format", &capital),
    ];
    let (ok, out) = run(dir, &projects, false);
    let rows: Vec<String> = out.lines().skip(1).map(str::to_string).collect();
    let (_, summary) = run(dir, &projects, true);
    let summary: Vec<&str> = summary.lines().collect();
    let want_summary = [
        SUMMARY,
        "commit-attribution\tr1\t2\t2\t2\t0",
        "commit-format\tr1\t2\t2\t3\t0",
    ];
    if sorted(rows.clone()) != sorted(expected.clone()) || summary != want_summary {
        println!("{out}{}", summary.join("\n"));
    }
    vec![
        ("the commit rules succeed".to_string(), ok),
        ("each commit of a session is checked once, against the latest-starting session; merges, stashes and the user's commits are skipped; a scope added on the branch counts".to_string(), sorted(rows) == sorted(expected)),
        ("the commit rules apply to the sessions with a checked commit".to_string(), summary == want_summary),
    ]
}

const FRENCH: &str = "Le livrable est fait : les tests passent et la revue ne trouve rien, donc je committe puis je passe au suivant dans la foulée.";
const ENGLISH: &str = "The deliverable is done: the tests pass and the review finds nothing, so I commit it and move on to the next one with the plan.";

// spec: BRE-10
fn language_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("language");
    let projects = tmp.join("projects");
    let mut expected = Vec::new();
    for (name, language) in [
        ("fr", Some("French")),
        ("de", Some("German")),
        ("none", None),
    ] {
        let repo = new_repo(&tmp.join(name));
        let key = language
            .map(|l| format!("user_language = \"{l}\"\n"))
            .unwrap_or_default();
        commit_at(
            &repo,
            "2026-01-01T00:00:00Z",
            "chore: sync",
            &[(
                ".claude/harness.toml",
                &format!("[harness]\nsource = \"x\"\nrev = \"r1\"\n\n[project]\n{key}"),
            )],
        );
        let mut t = Transcript::new(&repo, "2026-02-01T10:00:00.000Z");
        t.typed("SECRET");
        t.text("m1", FRENCH);
        let english = t.text("m2", ENGLISH);
        t.text("m3", "The tests pass, so I commit it and move on.");
        t.text(
            "m4",
            &format!("Voici le diff :\n```\n{ENGLISH}\n```\net `{ENGLISH}` fini."),
        );
        t.write(&projects.join(format!("-{name}/s.jsonl")));
        let mut sub = Transcript::new(&repo, "2026-02-01T10:01:00.000Z");
        sub.text("m5", ENGLISH);
        sub.write(&projects.join(format!("-{name}/s/subagents/agent-y.jsonl")));
        if name == "fr" {
            expected.push(format!(
                "s\tfr\tr1\tuser-language\tbreach\ts.jsonl:{english}\t"
            ));
        }
    }
    let (ok, out) = run(dir, &projects, false);
    let rows: Vec<String> = out.lines().skip(1).map(str::to_string).collect();
    let (_, summary) = run(dir, &projects, true);
    let summary: Vec<&str> = summary.lines().collect();
    let want_summary = [SUMMARY, "user-language\tr1\t1\t1\t1\t0"];
    if rows != expected || summary != want_summary {
        println!("{out}{}", summary.join("\n"));
    }
    vec![
        ("the language rule succeeds".to_string(), ok && !out.contains("SECRET") && !out.contains("deliverable")),
        ("a long block of the session in the other language is a breach; short, code and sub-agent blocks are not".to_string(), rows == expected),
        ("the language rule applies only with a French or English user_language".to_string(), summary == want_summary),
    ]
}

#[test]
fn rule_breaches() {
    let dir = std::path::PathBuf::from("/tools");
    let temp = tempfile::tempdir().unwrap();
    let tmp = temp.path().to_path_buf();
    let checks: Vec<_> = finding_checks(&dir, &tmp)
        .into_iter()
        .chain(summary_checks(&dir, &tmp))
        .chain(commit_checks(&dir, &tmp))
        .chain(language_checks(&dir, &tmp))
        .collect();
    let mut failed = false;
    for (name, ok) in checks {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    }
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
