//! Behavior test for `harness tool delivered-quality` on a fixture repository: the commits of each done issue, the fixes and
//! reverts of its files after the close, its deliverables, revision and reviews, the failed checks and typed messages
//! of its sessions, no text of a commit, an issue or a message.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const COLUMNS: &str = "issue\trevision\tclosed\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\tsessions\tfailed_checks\tvalidations\tother_messages";

fn sh(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .is_ok_and(|o| o.status.success());
    assert!(ok, "git {args:?} failed in {}", dir.display());
}

/// Writes `files` (path, content; `None` removes it) in `repo` and commits them at `date` with `message`; returns the
/// full hash.
fn commit_at(repo: &Path, date: &str, message: &str, files: &[(&str, Option<&str>)]) -> String {
    for (path, body) in files {
        let file = repo.join(path);
        match body {
            Some(body) => {
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                std::fs::write(file, body).unwrap();
            }
            None => std::fs::remove_file(file).unwrap(),
        }
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
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&head.stdout).trim().to_string()
}

fn toml(rev: &str) -> String {
    format!("[harness]\nsource = \"https://example.org/harness\"\nrev = \"{rev}\"\n")
}

const ALPHA: &str = "# SECRET alpha\n\nRun the code review.\n\n# Tracking\n\n## Plan\n1. [x] SECRET one\n2. [x] two\n   3. [ ] not a plan line\n\n## Decisions\n1. [x] not in the plan\n\n## Deliverable 1\n```sh\n# a comment, not a heading\n```\n| code review | low |\n- Code review: 6 findings, 1 finding dismissed.\n- code review (low): 1 finding; second review: no findings\n- Code review: no findings.\n";

/// The fixture repository: `alpha` (three commits, one fix inside, fixes and reverts after), `beta` (closed in one
/// commit that also rewrites a citation in `alpha`), `gamma` (done in the work tree only).
fn fixture(tmp: &Path) -> PathBuf {
    let repo = tmp.join("proj");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    commit_at(
        &repo,
        "2026-01-01T00:00:00Z",
        "chore: sync",
        &[(".claude/harness.toml", Some(&toml("r1")))],
    );
    commit_at(
        &repo,
        "2026-01-02T00:00:00Z",
        "docs: open the issues",
        &[
            ("issues/alpha.md", Some("# SECRET alpha\n")),
            ("issues/beta.md", Some("# beta\n")),
        ],
    );
    commit_at(
        &repo,
        "2026-01-03T00:00:00Z",
        "feat(files): SECRET a",
        &[
            ("src/a.rs", Some("a")),
            ("docs/arch.md", Some("a")),
            (".claude/harness.lock", Some("a")),
            ("issues/alpha.md", Some(ALPHA)),
        ],
    );
    let gap = commit_at(
        &repo,
        "2026-01-04T00:00:00Z",
        "fix(files): gap",
        &[
            ("src/a.rs", Some("b")),
            ("issues/alpha.md", Some(&format!("{ALPHA}x\n"))),
        ],
    );
    commit_at(
        &repo,
        "2026-01-04T06:00:00Z",
        "chore: bump",
        &[(".claude/harness.toml", Some(&toml("r2")))],
    );
    commit_at(&repo, "2026-01-04T12:00:00Z", "docs: close beta", &[
        ("issues/beta.md", None),
        ("issues/done/beta.md", Some("# beta\n\n# Tracking\n\n1. [x] one\n2. [x] two\n\n## Decisions\n1. [x] not in the plan\n* Code review: 2 findings.\n* Code review: fixed the doc.\n")),
        ("issues/alpha.md", Some(&format!("{ALPHA}y\n"))),
        (".claude/harness.toml", Some(&toml("r3"))),
    ]);
    commit_at(
        &repo,
        "2026-01-05T00:00:00Z",
        "docs: close alpha",
        &[
            ("issues/alpha.md", None),
            ("issues/done/alpha.md", Some(ALPHA)),
        ],
    );
    commit_at(
        &repo,
        "2026-01-10T00:00:00Z",
        "fix(files): later",
        &[("src/a.rs", Some("c"))],
    );
    commit_at(
        &repo,
        "2026-01-10T01:00:00Z",
        "fix: docs and lock only",
        &[
            ("docs/arch.md", Some("b")),
            (".claude/harness.lock", Some("b")),
        ],
    );
    commit_at(
        &repo,
        "2026-01-11T00:00:00Z",
        "chore: undo",
        &[("src/b.rs", Some("x"))],
    );
    commit_at(
        &repo,
        "2026-01-11T01:00:00Z",
        &format!("chore: undo the gap\n\nThis reverts commit {gap}."),
        &[("src/b.rs", Some("y"))],
    );
    commit_at(
        &repo,
        "2026-01-12T00:00:00Z",
        "Revert \"feat(files): a\"",
        &[("src/a.rs", Some("d"))],
    );
    commit_at(
        &repo,
        "2026-01-13T00:00:00Z",
        "revert(files): a",
        &[("src/a.rs", Some("e"))],
    );
    commit_at(
        &repo,
        "2026-02-20T00:00:00Z",
        "fix(files): too late",
        &[("src/a.rs", Some("f"))],
    );
    commit_at(
        &repo,
        "2026-01-04T23:00:00Z",
        "fix(files): rebased after the close",
        &[("src/a.rs", Some("h"))],
    );
    sh(&repo, &["checkout", "-q", "-b", "side"]);
    commit_at(
        &repo,
        "2026-01-09T00:00:00Z",
        "fix(files): on a side branch",
        &[("src/a.rs", Some("g"))],
    );
    sh(&repo, &["checkout", "-q", "main"]);
    std::fs::write(repo.join("issues/done/gamma.md"), "# gamma\n\n# Tracking\n").unwrap();
    repo
}

/// Runs the script on `repo`; with no `--projects` in `args`, on a projects directory that does not exist, so the
/// real transcripts are never read.
fn run(dir: &Path, repo: &Path, args: &[&str]) -> (bool, String) {
    let mut cmd = tool_command(dir.join("delivered_quality.rs"));
    cmd.arg(repo).args(args);
    if !args.contains(&"--projects") {
        cmd.arg("--projects").arg(repo.join("no-projects"));
    }
    let out = cmd.output().unwrap();
    if !out.status.success() {
        println!("{}", String::from_utf8_lossy(&out.stderr));
    }
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// The rows of `out` by issue, each a map from column to value.
fn rows(out: &str) -> HashMap<String, HashMap<String, String>> {
    let columns: Vec<&str> = COLUMNS.split('\t').collect();
    out.lines()
        .skip(1)
        .map(|l| {
            let row: HashMap<String, String> = columns
                .iter()
                .zip(l.split('\t'))
                .map(|(c, v)| (c.to_string(), v.to_string()))
                .collect();
            (row["issue"].clone(), row)
        })
        .collect()
}

fn has(
    rows: &HashMap<String, HashMap<String, String>>,
    issue: &str,
    column: &str,
    value: &str,
) -> bool {
    let got = rows
        .get(issue)
        .and_then(|r| r.get(column))
        .map(String::as_str);
    if got != Some(value) {
        println!("{issue}.{column}: expected {value:?}, got {got:?}");
    }
    got == Some(value)
}

// spec: QUA-1, QUA-2, QUA-3, QUA-4, QUA-5, QUA-10
fn git_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = fixture(tmp);
    let (ok, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    let issues: Vec<&str> = out
        .lines()
        .skip(1)
        .map(|l| l.split('\t').next().unwrap_or_default())
        .collect();
    let (_, wide) = run(dir, &repo, &["--window-days", "60"]);
    let w = rows(&wide);
    vec![
        (
            "the script runs and prints the header".to_string(),
            ok && out.lines().next() == Some(COLUMNS),
        ),
        (
            "one row per closed done issue, sorted by close date".to_string(),
            issues == ["beta", "alpha"],
        ),
        (
            "every row has every column".to_string(),
            out.lines()
                .all(|l| l.split('\t').count() == COLUMNS.split('\t').count()),
        ),
        (
            "the close date is the author date of the close commit".to_string(),
            has(&r, "alpha", "closed", "2026-01-05T00:00:00+00:00"),
        ),
        (
            "the commits run from the first with a Tracking to the close, another close excluded"
                .to_string(),
            has(&r, "alpha", "commits", "3") && has(&r, "beta", "commits", "1"),
        ),
        (
            "the deliverables are the plan lines of the done file, under Tracking when no Plan"
                .to_string(),
            has(&r, "alpha", "deliverables", "2") && has(&r, "beta", "deliverables", "2"),
        ),
        (
            "the fixes inside the issue are counted".to_string(),
            has(&r, "alpha", "fixes_in_issue", "1") && has(&r, "beta", "fixes_in_issue", "0"),
        ),
        (
            "a later fix of its files counts, docs, lock, side branch and window excluded"
                .to_string(),
            has(&r, "alpha", "fixes_after", "2") && has(&r, "beta", "fixes_after", "0"),
        ),
        (
            "the window can be widened".to_string(),
            has(&w, "alpha", "fixes_after", "3"),
        ),
        (
            "a negative window and a --projects with no value are refused".to_string(),
            !run(dir, &repo, &["--window-days", "-1"]).0 && !run(dir, &repo, &["--projects"]).0,
        ),
        (
            "reverts by type, by subject and by cited hash are counted".to_string(),
            has(&r, "alpha", "reverts_after", "3"),
        ),
        (
            "the revision is the one in force before the first commit of the issue".to_string(),
            has(&r, "alpha", "revision", "r1") && has(&r, "beta", "revision", "r2"),
        ),
        (
            "no text of a commit or an issue is printed".to_string(),
            !out.contains("SECRET"),
        ),
    ]
}

// spec: QUA-6
fn review_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = fixture(&tmp.join("reviews"));
    let (_, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    vec![
        (
            "each Code review line of the Tracking counts a review".to_string(),
            has(&r, "alpha", "reviews", "3") && has(&r, "beta", "reviews", "2"),
        ),
        (
            "the findings of the reviews are summed".to_string(),
            has(&r, "alpha", "review_findings", "7"),
        ),
        (
            "a review whose findings are not counted leaves the sum empty".to_string(),
            has(&r, "beta", "review_findings", ""),
        ),
    ]
}

/// A transcript line of `kind` in `cwd` at `ts`, with `fields` merged in.
fn line(kind: &str, cwd: &Path, ts: &str, fields: serde_json::Value) -> String {
    let mut line = serde_json::json!({"type": kind, "cwd": cwd, "timestamp": ts, "sessionId": "x", "gitBranch": "main"});
    line.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    format!("{line}\n")
}

fn typed(cwd: &Path, ts: &str, text: &str) -> String {
    line(
        "user",
        cwd,
        ts,
        serde_json::json!({"message": {"role": "user", "content": text}}),
    )
}

/// A `Bash` call at `ts` and its result a second later, an error when `error`.
fn bash(cwd: &Path, ts: &str, id: &str, command: &str, error: bool) -> String {
    let call = line(
        "assistant",
        cwd,
        &format!("{ts}:00Z"),
        serde_json::json!({"message": {"id": id, "content": [{"type": "tool_use", "id": id, "name": "Bash", "input": {"command": command}}]}}),
    );
    let result = line(
        "user",
        cwd,
        &format!("{ts}:01Z"),
        serde_json::json!({"message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": id, "is_error": error, "content": "SECRET output"}]}}),
    );
    call + &result
}

fn write(path: &Path, lines: &[String]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, lines.concat()).unwrap();
}

// spec: QUA-7, QUA-8
fn session_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("sessions");
    let repo = fixture(&tmp);
    let projects = tmp.join("projects");
    // S0, on another branch, starts after S1 and holds the first commit of alpha too: S1, on alpha's branch, wins.
    write(
        &projects.join("-proj/s0.jsonl"),
        &[
            typed(&repo, "2026-01-02T20:00:00Z", "SECRET early"),
            bash(&repo, "2026-01-03T10:00", "s0a", "just check", true),
        ],
    );
    // S1 runs on the branch of alpha from before the issues were opened: all its span up to alpha's last commit counts.
    let s1 = [
        typed(&repo, "2026-01-01T23:00:00Z", "SECRET prepare alpha"),
        typed(&repo, "2026-01-02T23:00:00Z", "SECRET do alpha"),
        bash(&repo, "2026-01-03T01:00", "a1", "just check", true),
        bash(&repo, "2026-01-03T02:00", "a2", "just test-harness", false),
        bash(&repo, "2026-01-03T03:00", "a3", "cargo check", true),
        bash(&repo, "2026-01-03T03:30", "a4", "echo just a check", true),
        bash(
            &repo,
            "2026-01-04T05:00",
            "a5",
            "cd x && just check-harness",
            true,
        ),
        typed(&repo, "2026-01-04T08:00:00Z", "ok"),
        bash(&repo, "2026-01-04T09:00", "a6", "just check", true),
        typed(&repo, "2026-01-05T00:30:00Z", "1 a, 2 b"),
        typed(&repo, "2026-01-05T01:00:00Z", "SECRET after"),
    ];
    write(
        &projects.join("-proj/s1.jsonl"),
        &s1.map(|l| l.replace("\"gitBranch\":\"main\"", "\"gitBranch\":\"alpha\"")),
    );
    write(
        &projects.join("-proj/s1/subagents/agent-1.jsonl"),
        &[bash(&repo, "2026-01-03T04:00", "b1", "just check", true)],
    );
    let (ok, out) = run(dir, &repo, &["--projects", projects.to_str().unwrap()]);
    let r = rows(&out);
    let (_, none) = run(dir, &repo, &[]);
    let n = rows(&none);
    vec![
        ("the script reads the transcripts".to_string(), ok),
        ("the sessions of an issue are the latest-starting ones holding its commits, its branch first".to_string(), has(&r, "alpha", "sessions", "1") && has(&r, "beta", "sessions", "1")),
        ("the failed just checks of its span are counted, not other commands nor sub-agents".to_string(), has(&r, "alpha", "failed_checks", "3") && has(&r, "beta", "failed_checks", "1")),
        ("the typed messages of its span are split into validations and the rest".to_string(), has(&r, "alpha", "validations", "1") && has(&r, "alpha", "other_messages", "2") && has(&r, "beta", "validations", "1") && has(&r, "beta", "other_messages", "0")),
        ("the transcript columns are empty when the projects cannot be read".to_string(), ["sessions", "failed_checks", "validations", "other_messages"].iter().all(|c| has(&n, "alpha", c, ""))),
        ("no text of a message is printed".to_string(), !out.contains("SECRET")),
    ]
}

// spec: QUA-9
fn summary_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let tmp = tmp.join("summary");
    let repo = fixture(&tmp);
    // gamma, closed in the revision of beta, makes r2 a sum of two issues; its copy left in the work tree goes first.
    std::fs::remove_file(repo.join("issues/done/gamma.md")).unwrap();
    commit_at(
        &repo,
        "2026-02-01T00:00:00Z",
        "chore: unpin back",
        &[(".claude/harness.toml", Some(&toml("r2")))],
    );
    commit_at(
        &repo,
        "2026-02-02T00:00:00Z",
        "feat(files): gamma",
        &[
            (
                "issues/done/gamma.md",
                Some("# gamma\n\n# Tracking\n\n1. [x] one\n- Code review: 2 findings\n"),
            ),
            ("src/g.rs", Some("g")),
        ],
    );
    // delta, in a revision named before the others, comes last: the revisions are in the order they were used.
    commit_at(
        &repo,
        "2026-02-03T00:00:00Z",
        "chore: pin",
        &[(".claude/harness.toml", Some(&toml("a0")))],
    );
    commit_at(
        &repo,
        "2026-02-04T00:00:00Z",
        "feat(files): delta",
        &[(
            "issues/done/delta.md",
            Some("# delta\n\n# Tracking\n\n1. [x] one\n"),
        )],
    );
    let (ok, out) = run(dir, &repo, &["--summary"]);
    let expected = [
        "revision\tissues\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\tsessions\tfailed_checks\tvalidations\tother_messages",
        "r1\t1\t2\t3\t1\t2\t3\t3\t7\t\t\t\t",
        "r2\t2\t3\t2\t0\t0\t0\t3\t\t\t\t\t",
        "a0\t1\t1\t1\t0\t0\t0\t0\t0\t\t\t\t",
    ];
    let lines: Vec<&str> = out.lines().collect();
    if lines != expected {
        println!("{out}");
    }
    vec![("the summary sums the columns per revision in the order they were used, an empty value leaving its sum empty".to_string(), ok && lines == expected)]
}

#[test]
fn delivered_quality() {
    let dir = std::path::PathBuf::from("/tools");
    let temp = tempfile::tempdir().unwrap();
    let tmp = temp.path().to_path_buf();
    let checks: Vec<_> = git_checks(&dir, &tmp)
        .into_iter()
        .chain(review_checks(&dir, &tmp))
        .chain(session_checks(&dir, &tmp))
        .chain(summary_checks(&dir, &tmp))
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
