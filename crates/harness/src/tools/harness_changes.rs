//! List the expected effects recorded in the history of the harness source (`specs/harness-verdicts.md`): one row
//! per `Expected-effect: <metric> <up|down>` line of a commit that changes `files/`, with the revisions in use (read
//! from the `session-metrics` rows) that hold the change, so `harness-report --changes` can give each a verdict.
//! Prints no commit text but the metric and the direction.
//! Usage: `harness run harness-changes [repo] --sessions <session-metrics rows>`.

use std::collections::HashMap;
use std::path::Path;

use super::session_metrics::HEADER as SESSIONS;
use super::transcripts::{default_branch, git};

const HEADER: &str = "change\tmetric\tdirection\trevisions";
const USAGE: &str = "usage: harness run harness-changes [repo] --sessions <file>";
const PREFIX: &str = "Expected-effect:";
/// The metrics an expectation can name: columns of the `harness-report` tables (`breached_share:<rule>` apart).
pub(super) const METRICS: [&str; 9] = [
    "turns",
    "tool_calls",
    "output_tokens",
    "input_tokens",
    "validations",
    "other_messages",
    "fixes_after_per_issue",
    "findings_per_review",
    "total_bytes",
];
pub(super) const RULE_METRIC: &str = "breached_share:";

/// The metric and direction of an `Expected-effect:` line; `None` for another line, `Some(Err(()))` for one that
/// names no known metric and direction.
pub(super) fn expectation(line: &str) -> Option<Result<(&str, &str), ()>> {
    let rest = line.strip_prefix(PREFIX)?;
    let words: Vec<&str> = rest.split_whitespace().collect();
    let known = |metric: &str| {
        METRICS.contains(&metric)
            || metric.strip_prefix(RULE_METRIC).is_some_and(|rule| {
                !rule.is_empty()
                    && rule
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            })
    };
    Some(match words[..] {
        [metric, direction @ ("up" | "down")] if rest.starts_with(' ') && known(metric) => {
            Ok((metric, direction))
        }
        _ => Err(()),
    })
}

/// The distinct revisions of the sessions file at `path`, in the order of their earliest start; `unpinned` and the
/// empty revision left out.
fn revisions(path: &str) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = text.lines();
    if lines.next() != Some(SESSIONS) {
        return Err(format!(
            "{path} has other columns than expected: give it the output of session-metrics"
        ));
    }
    let columns: Vec<&str> = SESSIONS.split('\t').collect();
    let at = |name: &str| columns.iter().position(|c| *c == name).unwrap_or(0);
    let (revision, start) = (at("revision"), at("start"));
    let mut first: HashMap<String, String> = HashMap::new();
    for line in lines.filter(|l| !l.is_empty()) {
        let cells: Vec<&str> = line.split('\t').collect();
        let (Some(rev), Some(when)) = (cells.get(revision), cells.get(start)) else {
            continue;
        };
        if rev.is_empty() || *rev == "unpinned" {
            continue;
        }
        let earliest = first
            .entry((*rev).to_string())
            .or_insert((*when).to_string());
        if *when < earliest.as_str() {
            *earliest = (*when).to_string();
        }
    }
    let mut ordered: Vec<(String, String)> = first.into_iter().collect();
    ordered.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    Ok(ordered.into_iter().map(|(rev, _)| rev).collect())
}

/// The expectations of the non-merge commits of the first-parent history of the default branch that change `files/`,
/// sorted by commit date then by line: (change, metric, direction).
fn expectations(repo: &Path) -> Vec<(String, String, String)> {
    let Some(branch) = default_branch(repo) else {
        return Vec::new();
    };
    let log = git(
        repo,
        &[
            "log",
            "--first-parent",
            "--no-merges",
            "--format=%H%x00%ct%x00%B%x1e",
            branch,
            "--",
            "files",
        ],
    )
    .unwrap_or_default();
    let mut commits: Vec<(i64, String, &str)> = log
        .split('\x1e')
        .filter_map(|entry| {
            let mut parts = entry.trim_start_matches('\n').splitn(3, '\0');
            let hash = parts.next()?.to_string();
            let date = parts.next()?.parse().ok()?;
            Some((date, hash, parts.next()?))
        })
        .collect();
    commits.sort_by_key(|(date, ..)| *date);
    commits
        .into_iter()
        .flat_map(|(_, hash, body)| {
            body.lines()
                .filter_map(|line| expectation(line)?.ok())
                .map(|(metric, direction)| {
                    (hash.clone(), metric.to_string(), direction.to_string())
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn options(args: &[String]) -> Result<(String, String), String> {
    let (mut repo, mut sessions) = (None, None);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sessions" => sessions = Some(args.next().ok_or("--sessions needs a file")?.clone()),
            flag if flag.starts_with('-') => return Err(format!("unknown option {flag}")),
            path if repo.is_none() => repo = Some(path.to_string()),
            extra => return Err(format!("unexpected argument {extra}")),
        }
    }
    Ok((
        repo.unwrap_or_else(|| ".".into()),
        sessions.ok_or("--sessions is required")?,
    ))
}

pub(super) fn main(args: &[String]) -> u8 {
    let fail = |e: String| {
        eprintln!("error: {e}\n{USAGE}");
        2
    };
    let (repo, sessions) = match options(args) {
        Ok(options) => options,
        Err(e) => return fail(e),
    };
    let repo = Path::new(&repo);
    if git(repo, &["rev-parse", "--git-dir"]).is_none() {
        return fail(format!("{} is not a git repository", repo.display()));
    }
    let revisions = match revisions(&sessions) {
        Ok(revisions) => revisions,
        Err(e) => return fail(e),
    };
    let commits: Vec<(&String, Option<String>)> = revisions
        .iter()
        .map(|rev| {
            let commit = rev.strip_prefix("self:").unwrap_or(rev);
            (
                rev,
                git(
                    repo,
                    &[
                        "rev-parse",
                        "--verify",
                        "-q",
                        &format!("{commit}^{{commit}}"),
                    ],
                ),
            )
        })
        .collect();
    let mut held: HashMap<String, String> = HashMap::new();
    println!("{HEADER}");
    for (change, metric, direction) in expectations(repo) {
        let revisions = held.entry(change.clone()).or_insert_with(|| {
            commits
                .iter()
                .filter_map(|(rev, commit)| {
                    let commit = commit.as_deref()?;
                    let ancestor = std::process::Command::new("git")
                        .arg("-C")
                        .arg(repo)
                        .args(["merge-base", "--is-ancestor", &change, commit])
                        .status()
                        .is_ok_and(|s| s.success());
                    ancestor.then_some(rev.as_str())
                })
                .collect::<Vec<_>>()
                .join(",")
        });
        println!("{change}\t{metric}\t{direction}\t{revisions}");
    }
    0
}
