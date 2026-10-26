#!/usr/bin/env rust-script
//! Measure what happened to the work of each done issue of a repository (`specs/delivered-quality.md`): one
//! tab-separated row per file of `issues/done/`, with the commits of the issue, the fixes inside it, and the fixes
//! and reverts of its files on the default branch after its close. Signals, not verdicts: a fix after a feature may
//! come from a new need. No text of a commit or an issue is printed.
//! Usage: `rust-script delivered_quality.rs [repo] [--window-days <n>]` (default the current directory and 30 days).
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! chrono = "0.4"
//! toml = "0.8"
//! ```

// transcripts.rs sha256: 26c36654f5330907f806aa94578bfc0c84c1ce0f54962ec8b259759e75ce8a4d
#[path = "transcripts.rs"]
mod transcripts;

use chrono::{DateTime, Duration, FixedOffset};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use transcripts::{default_branch, git, revision, sorted_entries};

const HEADER: &str = "issue\trevision\tclosed\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\tsessions\tfailed_checks\tvalidations\tother_messages";

/// The paths whose later fixes say nothing of the issue's work: the issue files, the docs and the lock of the sync.
const UNRELATED_PREFIXES: &[&str] = &["issues/", "docs/"];
const UNRELATED_FILES: &[&str] = &[".claude/harness.lock"];

/// A non-merge commit of the default branch.
struct Commit {
    hash: String,
    date: DateTime<FixedOffset>,
    subject: String,
    body: String,
    /// The paths it changed, renames read as a deletion and an addition.
    paths: Vec<String>,
    /// The paths it added.
    added: Vec<String>,
}

impl Commit {
    /// The Conventional Commits type of its subject (`AGENTS.md`), empty when it has none.
    fn kind(&self) -> &str {
        let head = self.subject.split(':').next().unwrap_or_default();
        let kind = head.split('(').next().unwrap_or_default().trim_end_matches('!');
        if self.subject.contains(':') && !kind.is_empty() && kind.chars().all(|c| c.is_ascii_lowercase()) { kind } else { "" }
    }

    fn touches(&self, path: &str) -> bool {
        self.paths.iter().any(|p| p == path)
    }
}

/// The non-merge commits of the default branch of `repo`, oldest first.
fn commits(repo: &Path) -> Vec<Commit> {
    let Some(branch) = default_branch(repo) else { return Vec::new() };
    let Some(log) = git(repo, &["log", "--no-merges", "--no-renames", "--reverse", "--name-status", "--format=%x1e%H%x1f%aI%x1f%B%x1f", branch]) else { return Vec::new() };
    log.split('\x1e')
        .filter_map(|record| {
            let mut fields = record.splitn(4, '\x1f');
            let hash = fields.next()?.trim().to_string();
            let date = DateTime::parse_from_rfc3339(fields.next()?).ok()?;
            let message = fields.next()?;
            let status = fields.next().unwrap_or_default();
            let mut paths = Vec::new();
            let mut added = Vec::new();
            for line in status.lines().filter(|l| !l.trim().is_empty()) {
                let mut parts = line.split('\t');
                let code = parts.next().unwrap_or_default();
                if let Some(path) = parts.next() {
                    if code == "A" {
                        added.push(path.to_string());
                    }
                    paths.push(path.to_string());
                }
            }
            Some(Commit {
                hash,
                date,
                subject: message.lines().next().unwrap_or_default().to_string(),
                body: message.to_string(),
                paths,
                added,
            })
        })
        .collect()
}

/// The indexes in `log` of the commits of the issue `stem` (QUA-2), oldest first; empty when it was never closed on
/// the default branch.
fn issue_commits(repo: &Path, log: &[Commit], stem: &str) -> Vec<usize> {
    let open = format!("issues/{stem}.md");
    let done = format!("issues/done/{stem}.md");
    let Some(close) = log.iter().rposition(|c| c.added.contains(&done)) else { return Vec::new() };
    let candidates: Vec<usize> = (0..=close).filter(|&i| log[i].touches(&open) || log[i].touches(&done)).collect();
    let tracked = |i: usize| {
        let show = |path: &str| git(repo, &["show", &format!("{}:{path}", log[i].hash)]);
        show(&done).or_else(|| show(&open)).is_some_and(|text| text.lines().any(|l| l.trim_end() == "# Tracking"))
    };
    let start = candidates.iter().copied().find(|&i| tracked(i)).unwrap_or(close);
    let closes_another = |c: &Commit| c.added.iter().any(|p| p.starts_with("issues/done/") && *p != done);
    candidates.into_iter().filter(|&i| i >= start && (i == close || !closes_another(&log[i]))).collect()
}

/// The number of plan lines of an issue file (QUA-3): `<n>. [x]` or `<n>. [ ]` at the start of a line of `## Plan`,
/// else of `# Tracking` before its first sub-heading (the older issues).
fn deliverables(text: &str) -> usize {
    let is_item = |line: &&str| {
        let rest = line.trim_start_matches(|c: char| c.is_ascii_digit());
        rest.len() < line.len() && (rest.starts_with(". [x]") || rest.starts_with(". [ ]"))
    };
    let section = |heading: &str| text.lines().skip_while(|l| l.trim_end() != heading).skip(1).take_while(|l| !l.starts_with('#')).filter(is_item).count();
    if text.lines().any(|l| l.trim_end() == "## Plan") { section("## Plan") } else { section("# Tracking") }
}

/// The reviews recorded in the `# Tracking` section of an issue file and the sum of their findings (QUA-6): a line
/// holding `Code review` is one review, its findings the `<n>` of its first `<n> finding(s)` (0 for `no findings`),
/// since what follows may count the fixed ones; `None` for the sum when a line gives no count.
fn reviews(text: &str) -> (usize, Option<usize>) {
    let tracking = text.lines().skip_while(|l| l.trim_end() != "# Tracking").take_while(|l| l.trim_end() == "# Tracking" || !l.starts_with("# "));
    let lines: Vec<String> = tracking.map(str::to_lowercase).filter(|l| l.contains("code review")).collect();
    let counts = lines.iter().map(|line| {
        let words: Vec<&str> = line.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
        words.windows(2).filter(|w| w[1] == "finding" || w[1] == "findings").find_map(|w| if w[0] == "no" { Some(0) } else { w[0].parse().ok() })
    });
    let sum = counts.collect::<Option<Vec<usize>>>().map(|c| c.iter().sum());
    (lines.len(), sum)
}

/// Whether `commit` reverts the work of the issue (QUA-4): a `revert` type or a `Revert "` subject changing one of
/// its `files`, or a `This reverts commit <hash>` citing one of its `own` commits.
fn reverts(commit: &Commit, files: &BTreeSet<&str>, own: &[&Commit]) -> bool {
    let typed = commit.kind() == "revert" || commit.subject.starts_with("Revert \"");
    let cited = commit.body.split("This reverts commit ").skip(1).any(|rest| {
        let hash: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
        hash.len() >= 7 && own.iter().any(|c| c.hash.starts_with(&hash))
    });
    cited || typed && commit.paths.iter().any(|p| files.contains(p.as_str()))
}

struct Row {
    issue: String,
    closed: DateTime<FixedOffset>,
    cells: Vec<String>,
}

fn issue_row(repo: &Path, log: &[Commit], path: &Path, window: Duration) -> Option<Row> {
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let own = issue_commits(repo, log, &stem);
    let (&first, &close) = (own.first()?, own.last()?);
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mine: Vec<&Commit> = own.iter().map(|&i| &log[i]).collect();
    let files: BTreeSet<&str> = mine
        .iter()
        .flat_map(|c| c.paths.iter().map(String::as_str))
        .filter(|p| !UNRELATED_PREFIXES.iter().any(|d| p.starts_with(d)) && !UNRELATED_FILES.contains(p))
        .collect();
    let end = log[close].date + window;
    let later: Vec<&Commit> = log[close + 1..].iter().filter(|c| c.date >= log[close].date && c.date <= end).collect();
    let fixes_after = later.iter().filter(|c| c.kind() == "fix" && c.paths.iter().any(|p| files.contains(p.as_str()))).count();
    let reverts_after = later.iter().filter(|c| reverts(c, &files, &mine)).count();
    let fixes_in = mine.iter().filter(|c| matches!(c.kind(), "fix" | "revert")).count();
    let (review_count, findings) = reviews(&text);
    let cells = vec![
        revision(repo, &(log[first].date - Duration::seconds(1)).to_rfc3339()),
        log[close].date.to_rfc3339(),
        deliverables(&text).to_string(),
        own.len().to_string(),
        fixes_in.to_string(),
        fixes_after.to_string(),
        reverts_after.to_string(),
        review_count.to_string(),
        findings.map(|n| n.to_string()).unwrap_or_default(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];
    Some(Row { issue: stem, closed: log[close].date, cells })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut repo = PathBuf::from(".");
    let mut window_days = 30;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--window-days" => match rest.next().and_then(|n| n.parse().ok()) {
                Some(n) => window_days = n,
                None => {
                    eprintln!("--window-days needs a number of days");
                    std::process::exit(2);
                }
            },
            other if other.starts_with("--") => {
                eprintln!("unknown option {other}");
                std::process::exit(2);
            }
            other => repo = PathBuf::from(other),
        }
    }
    let log = commits(&repo);
    let mut rows: Vec<Row> = sorted_entries(&repo.join("issues/done"))
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .filter_map(|p| issue_row(&repo, &log, p, Duration::days(window_days)))
        .collect();
    rows.sort_by(|a, b| a.closed.cmp(&b.closed).then_with(|| a.issue.cmp(&b.issue)));
    println!("{HEADER}");
    for row in rows {
        println!("{}\t{}", row.issue, row.cells.join("\t"));
    }
}
