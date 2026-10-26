#!/usr/bin/env rust-script
//! Measure what happened to the work of each done issue of a repository (`specs/delivered-quality.md`): one
//! tab-separated row per file of `issues/done/`, with the commits of the issue, the fixes inside it, and the fixes
//! and reverts of its files on the default branch after its close. Signals, not verdicts: a fix after a feature may
//! come from a new need. The sessions that made its commits give the failed checks and the typed messages of its
//! span. With `--summary`, the columns are summed per harness revision. No text of a commit, an issue or a message
//! is printed.
//! Usage: `rust-script delivered_quality.rs [repo] [--projects <dir>] [--window-days <n>] [--summary]` (default the current
//! directory, `~/.claude/projects` and 30 days).
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! chrono = "0.4"
//! toml = "0.8"
//! ```

// transcripts.rs sha256: 88aa683cf117f2087cfff20243255bd66990027890ee1f3f654b6f73590b0e85
#[path = "transcripts.rs"]
mod transcripts;

use chrono::{DateTime, Duration, FixedOffset};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use transcripts::{default_branch, for_each_line, git, is_validation, projects, projects_root, repository, revision, sessions, sorted_entries, subagents, typed_text, Place};

const SUMMARY_HEADER: &str = "revision\tissues\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\tsessions\tfailed_checks\tvalidations\tother_messages";
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
    let Some(log) = git(repo, &["-c", "core.quotePath=false", "log", "--no-merges", "--no-renames", "--reverse", "--name-status", "--format=%x1e%H%x1f%aI%x1f%B%x1f", branch]) else { return Vec::new() };
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
    // The commits whose diff of the file adds or removes the heading: the oldest of them added it.
    let branch = default_branch(repo).unwrap_or("HEAD");
    let tracked = git(repo, &["-c", "core.quotePath=false", "log", "--no-renames", "--format=%H", "-G^# Tracking$", branch, "--", &open, &done]).unwrap_or_default();
    let tracked: BTreeSet<&str> = tracked.lines().collect();
    let start = candidates.iter().copied().find(|&i| tracked.contains(log[i].hash.as_str())).unwrap_or(close);
    let closes_another = |c: &Commit| c.added.iter().any(|p| p.starts_with("issues/done/") && *p != done);
    candidates.into_iter().filter(|&i| i >= start && (i == close || !closes_another(&log[i]))).collect()
}

/// The lines of `text` after the line `heading`, up to the next line that `ends` accepts outside a fenced code block.
fn section<'a>(text: &'a str, heading: &str, ends: impl Fn(&str) -> bool) -> Vec<&'a str> {
    let mut fenced = false;
    text.lines()
        .skip_while(|l| l.trim_end() != heading)
        .skip(1)
        .take_while(|l| {
            fenced ^= l.starts_with("```");
            fenced || !ends(l)
        })
        .collect()
}

/// The number of plan lines of an issue file (QUA-3): `<n>. [x]` or `<n>. [ ]` at the start of a line of `## Plan`,
/// else of `# Tracking` before its first sub-heading (the older issues).
fn deliverables(text: &str) -> usize {
    let is_item = |line: &&str| {
        let rest = line.trim_start_matches(|c: char| c.is_ascii_digit());
        rest.len() < line.len() && (rest.starts_with(". [x]") || rest.starts_with(". [ ]"))
    };
    let heading = if text.lines().any(|l| l.trim_end() == "## Plan") { "## Plan" } else { "# Tracking" };
    section(text, heading, |l| l.starts_with('#')).into_iter().filter(is_item).count()
}

/// The reviews recorded in the `# Tracking` section of an issue file and the sum of their findings (QUA-6): a line
/// holding `Code review` is one review, unless it is a row of the `check | result` table, its findings the `<n>` of its first `<n> finding(s)` (0 for `no findings`),
/// since what follows may count the fixed ones; `None` for the sum when a line gives no count.
fn reviews(text: &str) -> (usize, Option<usize>) {
    let tracking = section(text, "# Tracking", |l| l.starts_with("# "));
    let lines: Vec<String> = tracking.iter().filter(|l| !l.trim_start().starts_with('|')).map(|l| l.to_lowercase()).filter(|l| l.contains("code review")).collect();
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

/// What a session tells of the issue it worked on, at a timestamp.
enum Event {
    Validation,
    Other,
    FailedCheck,
}

/// A session of the repository: its span, sub-agents included, its branches other than the default ones, and the
/// events of its own transcript.
struct Session {
    branches: Vec<String>,
    start: DateTime<FixedOffset>,
    end: DateTime<FixedOffset>,
    events: Vec<(DateTime<FixedOffset>, Event)>,
}

/// A check of the project: `just` followed by a word starting with `check` or `test` (QUA-8).
fn is_check(command: &str) -> bool {
    let words: Vec<&str> = command.split(|c: char| c.is_whitespace() || ";&|()".contains(c)).filter(|w| !w.is_empty()).collect();
    words.windows(2).any(|w| w[0] == "just" && (w[1].starts_with("check") || w[1].starts_with("test")))
}

/// The sessions of the projects directory `root` whose repository is `repo` (QUA-7); `None` when `root` cannot be read.
fn read_sessions(root: &Path, repo: &Path) -> Option<Vec<Session>> {
    std::fs::read_dir(root).ok()?;
    let repo = std::fs::canonicalize(repo).ok();
    let mut repositories: HashMap<String, Option<PathBuf>> = HashMap::new();
    let mut found = Vec::new();
    for path in projects(root).iter().flat_map(|p| sessions(p)) {
        let mut place = Place::default();
        let mut events = Vec::new();
        let mut checks: HashMap<String, bool> = HashMap::new();
        let read = for_each_line(&path, |line| {
            place.observe(line, true);
            let Some(at) = line["timestamp"].as_str().and_then(|t| DateTime::parse_from_rfc3339(t).ok()) else { return };
            if let Some(text) = typed_text(line) {
                events.push((at, if is_validation(&text) { Event::Validation } else { Event::Other }));
            }
            for block in line["message"]["content"].as_array().map(Vec::as_slice).unwrap_or_default() {
                let id = block["id"].as_str().or(block["tool_use_id"].as_str()).unwrap_or_default().to_string();
                if block["type"] == "tool_use" && block["name"] == "Bash" {
                    checks.insert(id, is_check(block["input"]["command"].as_str().unwrap_or_default()));
                } else if block["type"] == "tool_result" && block["is_error"] == Value::Bool(true) && checks.get(&id) == Some(&true) {
                    events.push((at, Event::FailedCheck));
                }
            }
        });
        if let Err(e) = read {
            eprintln!("cannot read {}: {e}", path.display());
        }
        for sub in subagents(&path) {
            let _ = for_each_line(&sub, |line| {
                place.observe(line, false);
            });
        }
        let (Some(cwd), Some((start, _)), Some(end)) = (place.cwd.clone(), place.start.clone(), place.end) else { continue };
        let resolved = repositories.entry(cwd.clone()).or_insert_with(|| repository(&cwd, &place.issues));
        if repo.is_some() && *resolved == repo {
            found.push(Session { branches: place.issues, start, end, events });
        }
    }
    Some(found)
}

/// The transcript columns of an issue (QUA-7, QUA-8): its sessions, then the failed checks, validations and other
/// typed messages of their spans that belong to it.
fn session_cells(log: &[Commit], own: &[usize], stem: &str, sessions: &[Session]) -> [usize; 4] {
    let on_branch = |s: &Session| s.branches.iter().any(|b| b == stem);
    let mut by_session: BTreeMap<usize, Vec<DateTime<FixedOffset>>> = BTreeMap::new();
    for &i in own {
        let at = log[i].date;
        let holding: Vec<(usize, &Session)> = sessions.iter().enumerate().filter(|(_, s)| s.start <= at && at <= s.end).collect();
        let preferred = holding.iter().filter(|(_, s)| on_branch(s)).max_by_key(|(_, s)| s.start);
        if let Some((k, _)) = preferred.or_else(|| holding.iter().max_by_key(|(_, s)| s.start)) {
            by_session.entry(*k).or_default().push(at);
        }
    }
    let mut cells = [by_session.len(), 0, 0, 0];
    for (k, dates) in &by_session {
        let session = &sessions[*k];
        let (Some(&first), Some(&last)) = (dates.iter().min(), dates.iter().max()) else { continue };
        let other_work = log.iter().enumerate().filter(|(i, c)| !own.contains(i) && c.date >= session.start && c.date < first).map(|(_, c)| c.date).max();
        let after = other_work.filter(|_| !on_branch(session));
        let inside = |at: &DateTime<FixedOffset>| after.map_or(*at >= session.start, |a| *at > a) && *at <= last;
        for (_, event) in session.events.iter().filter(|(at, _)| inside(at)) {
            cells[match event {
                Event::FailedCheck => 1,
                Event::Validation => 2,
                Event::Other => 3,
            }] += 1;
        }
    }
    cells
}

struct Row {
    issue: String,
    /// The author date of its oldest commit, which orders the revisions of the summary.
    first: DateTime<FixedOffset>,
    closed: DateTime<FixedOffset>,
    cells: Vec<String>,
}

fn issue_row(repo: &Path, log: &[Commit], sessions: Option<&[Session]>, path: &Path, window: Duration) -> Option<Row> {
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
    let later: Vec<&Commit> = log[close + 1..].iter().filter(|c| c.date <= end).collect();
    let fixes_after = later.iter().filter(|c| c.kind() == "fix" && c.paths.iter().any(|p| files.contains(p.as_str()))).count();
    let reverts_after = later.iter().filter(|c| reverts(c, &files, &mine)).count();
    let fixes_in = mine.iter().filter(|c| matches!(c.kind(), "fix" | "revert")).count();
    let (review_count, findings) = reviews(&text);
    let transcript = sessions.map(|s| session_cells(log, &own, &stem, s));
    let mut cells = vec![
        revision(repo, &(log[first].date - Duration::seconds(1)).to_rfc3339()),
        log[close].date.to_rfc3339(),
        deliverables(&text).to_string(),
        own.len().to_string(),
        fixes_in.to_string(),
        fixes_after.to_string(),
        reverts_after.to_string(),
        review_count.to_string(),
        findings.map(|n| n.to_string()).unwrap_or_default(),
    ];
    cells.extend((0..4).map(|k| transcript.map(|t| t[k].to_string()).unwrap_or_default()));
    Some(Row { issue: stem, first: log[first].date, closed: log[close].date, cells })
}

/// The rows summed per revision (QUA-9), in the order the revisions were first used: the count of issues, then the
/// sum of each numeric column, empty when one of its values is.
fn summary(rows: &[Row]) -> Vec<String> {
    let mut by_revision: HashMap<&str, (DateTime<FixedOffset>, usize, Vec<Option<u64>>)> = HashMap::new();
    for row in rows {
        let values: Vec<Option<u64>> = row.cells[2..].iter().map(|v| v.parse().ok()).collect();
        let (first, count, sums) = by_revision.entry(&row.cells[0]).or_insert_with(|| (row.first, 0, vec![Some(0); values.len()]));
        *first = (*first).min(row.first);
        *count += 1;
        for (sum, value) in sums.iter_mut().zip(values) {
            *sum = sum.zip(value).map(|(a, b)| a + b);
        }
    }
    let mut ordered: Vec<_> = by_revision.into_iter().collect();
    ordered.sort_by(|a, b| (a.1 .0, a.0).cmp(&(b.1 .0, b.0)));
    ordered
        .into_iter()
        .map(|(revision, (_, count, sums))| {
            let sums: Vec<String> = sums.iter().map(|s| s.map(|n| n.to_string()).unwrap_or_default()).collect();
            format!("{revision}\t{count}\t{}", sums.join("\t"))
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut repo = PathBuf::from(".");
    let mut window_days = 30;
    let mut projects_dir = None;
    let mut summed = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--window-days" => match rest.next().and_then(|n| n.parse::<i64>().ok()).filter(|n| (0..=3650).contains(n)) {
                Some(n) => window_days = n,
                None => {
                    eprintln!("--window-days needs a whole number of days from 0 to 3650");
                    std::process::exit(2);
                }
            },
            "--projects" => match rest.next() {
                Some(dir) => projects_dir = Some(dir.clone()),
                None => {
                    eprintln!("--projects needs a directory");
                    std::process::exit(2);
                }
            },
            "--summary" => summed = true,
            other if other.starts_with("--") => {
                eprintln!("unknown option {other}");
                std::process::exit(2);
            }
            other => repo = PathBuf::from(other),
        }
    }
    let log = commits(&repo);
    let sessions = projects_root(projects_dir.as_ref()).ok().and_then(|root| read_sessions(&root, &repo));
    let mut rows: Vec<Row> = sorted_entries(&repo.join("issues/done"))
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .filter_map(|p| issue_row(&repo, &log, sessions.as_deref(), p, Duration::days(window_days)))
        .collect();
    rows.sort_by(|a, b| a.closed.cmp(&b.closed).then_with(|| a.issue.cmp(&b.issue)));
    if summed {
        println!("{SUMMARY_HEADER}");
        summary(&rows).iter().for_each(|line| println!("{line}"));
        return;
    }
    println!("{HEADER}");
    for row in rows {
        println!("{}\t{}", row.issue, row.cells.join("\t"));
    }
}
