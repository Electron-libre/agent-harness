#!/usr/bin/env rust-script
//! Behavior test for `delivered_quality.rs` on a fixture repository: the commits of each done issue, the fixes and
//! reverts of its files after the close, its deliverables and revision, no text of a commit or an issue.
//! Usage: `just test-harness`.
//! ```cargo
//! [dependencies]
//! sha2 = "0.10"
//! tempfile = "3"
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const COLUMNS: &str = "issue\trevision\tclosed\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\tsessions\tfailed_checks\tvalidations\tother_messages";

fn sh(dir: &Path, args: &[&str]) {
    let ok = Command::new("git").args(args).current_dir(dir).output().is_ok_and(|o| o.status.success());
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
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", message])
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(out.status.success(), "commit failed: {}", String::from_utf8_lossy(&out.stderr));
    let head = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(repo).output().unwrap();
    String::from_utf8_lossy(&head.stdout).trim().to_string()
}

fn toml(rev: &str) -> String {
    format!("[harness]\nsource = \"https://example.org/harness\"\nrev = \"{rev}\"\n")
}

const ALPHA: &str = "# SECRET alpha\n\nRun the code review.\n\n# Tracking\n\n## Plan\n1. [x] SECRET one\n2. [x] two\n   3. [ ] not a plan line\n\n## Decisions\n1. [x] not in the plan\n\n## Deliverable 1\n- Code review: 6 findings, 1 finding dismissed.\n- code review (low): 1 finding; second review: no findings\n- Code review: no findings.\n";

/// The fixture repository: `alpha` (three commits, one fix inside, fixes and reverts after), `beta` (closed in one
/// commit that also rewrites a citation in `alpha`), `gamma` (done in the work tree only).
fn fixture(tmp: &Path) -> PathBuf {
    let repo = tmp.join("proj");
    std::fs::create_dir_all(&repo).unwrap();
    sh(&repo, &["init", "-q", "-b", "main"]);
    commit_at(&repo, "2026-01-01T00:00:00Z", "chore: sync", &[(".claude/harness.toml", Some(&toml("r1")))]);
    commit_at(&repo, "2026-01-02T00:00:00Z", "docs: open the issues", &[("issues/alpha.md", Some("# SECRET alpha\n")), ("issues/beta.md", Some("# beta\n"))]);
    commit_at(&repo, "2026-01-03T00:00:00Z", "feat(files): SECRET a", &[
        ("src/a.rs", Some("a")),
        ("docs/arch.md", Some("a")),
        (".claude/harness.lock", Some("a")),
        ("issues/alpha.md", Some(ALPHA)),
    ]);
    let gap = commit_at(&repo, "2026-01-04T00:00:00Z", "fix(files): gap", &[("src/a.rs", Some("b")), ("issues/alpha.md", Some(&format!("{ALPHA}x\n")))]);
    commit_at(&repo, "2026-01-04T06:00:00Z", "chore: bump", &[(".claude/harness.toml", Some(&toml("r2")))]);
    commit_at(&repo, "2026-01-04T12:00:00Z", "docs: close beta", &[
        ("issues/beta.md", None),
        ("issues/done/beta.md", Some("# beta\n\n# Tracking\n\n1. [x] one\n2. [x] two\n\n## Decisions\n1. [x] not in the plan\n* Code review: 2 findings.\n* Code review: fixed the doc.\n")),
        ("issues/alpha.md", Some(&format!("{ALPHA}y\n"))),
        (".claude/harness.toml", Some(&toml("r3"))),
    ]);
    commit_at(&repo, "2026-01-05T00:00:00Z", "docs: close alpha", &[("issues/alpha.md", None), ("issues/done/alpha.md", Some(ALPHA))]);
    commit_at(&repo, "2026-01-10T00:00:00Z", "fix(files): later", &[("src/a.rs", Some("c"))]);
    commit_at(&repo, "2026-01-10T01:00:00Z", "fix: docs and lock only", &[("docs/arch.md", Some("b")), (".claude/harness.lock", Some("b"))]);
    commit_at(&repo, "2026-01-11T00:00:00Z", "chore: undo", &[("src/b.rs", Some("x"))]);
    commit_at(&repo, "2026-01-11T01:00:00Z", &format!("chore: undo the gap\n\nThis reverts commit {gap}."), &[("src/b.rs", Some("y"))]);
    commit_at(&repo, "2026-01-12T00:00:00Z", "Revert \"feat(files): a\"", &[("src/a.rs", Some("d"))]);
    commit_at(&repo, "2026-01-13T00:00:00Z", "revert(files): a", &[("src/a.rs", Some("e"))]);
    commit_at(&repo, "2026-02-20T00:00:00Z", "fix(files): too late", &[("src/a.rs", Some("f"))]);
    sh(&repo, &["checkout", "-q", "-b", "side"]);
    commit_at(&repo, "2026-01-09T00:00:00Z", "fix(files): on a side branch", &[("src/a.rs", Some("g"))]);
    sh(&repo, &["checkout", "-q", "main"]);
    std::fs::write(repo.join("issues/done/gamma.md"), "# gamma\n\n# Tracking\n").unwrap();
    repo
}

fn run(dir: &Path, repo: &Path, args: &[&str]) -> (bool, String) {
    let out = Command::new("rust-script").arg(dir.join("delivered_quality.rs")).arg(repo).args(args).output().expect("rust-script must be installed");
    if !out.status.success() {
        println!("{}", String::from_utf8_lossy(&out.stderr));
    }
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The rows of `out` by issue, each a map from column to value.
fn rows(out: &str) -> HashMap<String, HashMap<String, String>> {
    let columns: Vec<&str> = COLUMNS.split('\t').collect();
    out.lines()
        .skip(1)
        .map(|l| {
            let row: HashMap<String, String> = columns.iter().zip(l.split('\t')).map(|(c, v)| (c.to_string(), v.to_string())).collect();
            (row["issue"].clone(), row)
        })
        .collect()
}

fn has(rows: &HashMap<String, HashMap<String, String>>, issue: &str, column: &str, value: &str) -> bool {
    let got = rows.get(issue).and_then(|r| r.get(column)).map(String::as_str);
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
    let issues: Vec<&str> = out.lines().skip(1).map(|l| l.split('\t').next().unwrap_or_default()).collect();
    let (_, wide) = run(dir, &repo, &["--window-days", "60"]);
    let w = rows(&wide);
    vec![
        ("the script runs and prints the header".to_string(), ok && out.lines().next() == Some(COLUMNS)),
        ("one row per closed done issue, sorted by close date".to_string(), issues == ["beta", "alpha"]),
        ("every row has every column".to_string(), out.lines().all(|l| l.split('\t').count() == COLUMNS.split('\t').count())),
        ("the close date is the author date of the close commit".to_string(), has(&r, "alpha", "closed", "2026-01-05T00:00:00+00:00")),
        ("the commits run from the first with a Tracking to the close, another close excluded".to_string(), has(&r, "alpha", "commits", "3") && has(&r, "beta", "commits", "1")),
        ("the deliverables are the plan lines of the done file, under Tracking when no Plan".to_string(), has(&r, "alpha", "deliverables", "2") && has(&r, "beta", "deliverables", "2")),
        ("the fixes inside the issue are counted".to_string(), has(&r, "alpha", "fixes_in_issue", "1") && has(&r, "beta", "fixes_in_issue", "0")),
        ("a later fix of its files counts, docs, lock, side branch and window excluded".to_string(), has(&r, "alpha", "fixes_after", "1") && has(&r, "beta", "fixes_after", "0")),
        ("the window can be widened".to_string(), has(&w, "alpha", "fixes_after", "2")),
        ("reverts by type, by subject and by cited hash are counted".to_string(), has(&r, "alpha", "reverts_after", "3")),
        ("the revision is the one in force before the first commit of the issue".to_string(), has(&r, "alpha", "revision", "r1") && has(&r, "beta", "revision", "r2")),
        ("no text of a commit or an issue is printed".to_string(), !out.contains("SECRET")),
    ]
}

// spec: QUA-6
fn review_checks(dir: &Path, tmp: &Path) -> Vec<(String, bool)> {
    let repo = fixture(&tmp.join("reviews"));
    let (_, out) = run(dir, &repo, &[]);
    let r = rows(&out);
    vec![
        ("each Code review line of the Tracking counts a review".to_string(), has(&r, "alpha", "reviews", "3") && has(&r, "beta", "reviews", "2")),
        ("the findings of the reviews are summed".to_string(), has(&r, "alpha", "review_findings", "7")),
        ("a review whose findings are not counted leaves the sum empty".to_string(), has(&r, "beta", "review_findings", "")),
    ]
}

// spec: QUA-10
fn stamp_checks(dir: &Path) -> Vec<(String, bool)> {
    use sha2::Digest;
    let script = std::fs::read_to_string(dir.join("delivered_quality.rs")).unwrap_or_default();
    let stamp = format!("// transcripts.rs sha256: {:x}", sha2::Sha256::digest(std::fs::read(dir.join("transcripts.rs")).unwrap_or_default()));
    if !script.contains(&stamp) {
        println!("delivered_quality.rs: write the line `{stamp}` so rust-script rebuilds it with the current transcripts.rs");
    }
    vec![
        ("delivered_quality.rs includes the shared module".to_string(), script.contains("#[path = \"transcripts.rs\"]")),
        ("delivered_quality.rs carries the SHA-256 of the current transcripts.rs".to_string(), script.contains(&stamp)),
    ]
}

fn main() {
    let dir = std::fs::canonicalize(Path::new(file!()).parent().unwrap()).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let tmp = temp.path().to_path_buf();
    let checks: Vec<_> = git_checks(&dir, &tmp).into_iter().chain(review_checks(&dir, &tmp)).chain(stamp_checks(&dir)).collect();
    let mut failed = false;
    for (name, ok) in checks {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    }
    if failed {
        std::process::exit(1);
    }
}
