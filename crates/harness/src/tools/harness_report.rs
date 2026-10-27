//! Print the evolution of the harness by revision (`specs/harness-report.md`): one markdown table per axis, one row
//! per group of consecutive revisions holding at least `--min-sessions` sessions, with the model, version and project
//! mix and the sample size next to the numbers. Reads the outputs of the other transcript readers, never the
//! transcripts; it reports trends, never a pass or a fail.
//! Usage: `harness run harness-report --sessions <session-metrics rows> [--breaches <rule-breaches --summary>]
//! [--quality <delivered-quality --summary>]... [--context <fixed-context rows>] [--min-sessions <n>] [--anonymous]`.

use std::collections::{BTreeMap, HashMap, HashSet};

const SESSIONS: &str = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields";
const BREACHES: &str = "rule\trevision\tapplied\tbreached\tbreaches\tblocked";
const QUALITY: &str = "revision\tissues\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\ttraced\ttraced_without_red\tsessions\tfailed_checks\tvalidations\tother_messages";
const CONTEXT: &str =
    "project\trevision\tcommit\tclaude_md_bytes\tagents_md_bytes\tskill_description_bytes\tskills";
const USAGE: &str = "usage: harness run harness-report --sessions <file> [--breaches <file>] [--quality <file>]... [--context <file>] [--min-sessions <n>] [--anonymous]";
const TOO_FEW: &str = "too few";

type Row = HashMap<String, String>;

/// The rows of a TSV file whose header must be `header`, each a map from column to value.
fn read(path: &str, header: &str) -> Result<Vec<Row>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err(format!(
            "{path} has other columns than expected: give it the output of the matching script"
        ));
    }
    let columns: Vec<&str> = header.split('\t').collect();
    Ok(lines
        .filter(|l| !l.is_empty())
        .map(|l| {
            columns
                .iter()
                .map(|c| c.to_string())
                .zip(l.split('\t').map(str::to_string))
                .collect()
        })
        .collect())
}

fn value<'a>(row: &'a Row, column: &str) -> &'a str {
    row.get(column).map(String::as_str).unwrap_or_default()
}

fn number(row: &Row, column: &str) -> u64 {
    value(row, column).parse().unwrap_or(0)
}

/// A revision as the report names it: a 40-digit hash, after `self:` or not, cut to its first 7 digits.
fn short(revision: &str) -> String {
    let (prefix, hash) = revision
        .strip_prefix("self:")
        .map_or(("", revision), |h| ("self:", h));
    if hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
        format!("{prefix}{}", &hash[..7])
    } else {
        revision.to_string()
    }
}

/// Consecutive revisions read together, so a comparison rests on enough sessions.
struct Group<'a> {
    name: String,
    revisions: HashSet<&'a str>,
    sessions: Vec<&'a Row>,
    too_few: bool,
}

/// The group of the revisions of `open`, in order, with their sessions.
fn close<'a>(open: Vec<(&'a str, Vec<&'a Row>)>, too_few: bool) -> Group<'a> {
    let (first, last) = (short(open[0].0), short(open[open.len() - 1].0));
    let name = if open.len() == 1 {
        first
    } else {
        format!("{first}..{last}")
    };
    Group {
        name,
        revisions: open.iter().map(|(r, _)| *r).collect(),
        sessions: open.into_iter().flat_map(|(_, s)| s).collect(),
        too_few,
    }
}

/// The revisions in the order of their first session, cut into groups closed at `min` sessions (REP-4).
fn groups(sessions: &[Row], min: usize) -> Vec<Group<'_>> {
    let mut by_revision: HashMap<&str, (&str, Vec<&Row>)> = HashMap::new();
    for row in sessions.iter().filter(|r| !value(r, "revision").is_empty()) {
        let entry = by_revision
            .entry(value(row, "revision"))
            .or_insert((value(row, "start"), Vec::new()));
        entry.0 = entry.0.min(value(row, "start"));
        entry.1.push(row);
    }
    let mut ordered: Vec<_> = by_revision.into_iter().collect();
    ordered.sort_by(|a, b| (a.1 .0, a.0).cmp(&(b.1 .0, b.0)));
    let mut groups: Vec<Group> = Vec::new();
    let mut open: Vec<(&str, Vec<&Row>)> = Vec::new();
    for (revision, (_, rows)) in ordered {
        open.push((revision, rows));
        if open.iter().map(|(_, s)| s.len()).sum::<usize>() >= min {
            groups.push(close(std::mem::take(&mut open), false));
        }
    }
    if !open.is_empty() {
        groups.push(close(open, true));
    }
    groups
}

/// The middle value, the mean of the two middle ones rounded half up for an even count; 0 for none.
fn median(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    let n = values.len();
    match n {
        0 => 0,
        _ if n % 2 == 1 => values[n / 2],
        _ => (values[n / 2 - 1] + values[n / 2]).div_ceil(2),
    }
}

/// `name=n` for each name of a comma-separated `column`, `n` the sessions that hold it, most frequent first.
fn mix(sessions: &[&Row], column: &str) -> String {
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for row in sessions {
        let names: HashSet<&str> = value(row, column)
            .split(',')
            .filter(|n| !n.is_empty())
            .collect();
        for name in names {
            *counts.entry(name).or_default() += 1;
        }
    }
    sorted_pairs(counts)
}

/// `name=n, …`, most frequent first, then by name.
fn sorted_pairs(counts: BTreeMap<&str, u64>) -> String {
    let mut pairs: Vec<_> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    pairs
        .iter()
        .map(|(name, n)| format!("{name}={n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A markdown table under `## <title>`: `group`, `sessions`, then `columns`, one row or more per group; a group with
/// too few sessions is one row showing only its size.
fn table(
    title: &str,
    columns: &[&str],
    groups: &[Group],
    cells: impl Fn(&Group) -> Vec<Vec<String>>,
) {
    println!("## {title}\n");
    println!("| group | sessions | {} |", columns.join(" | "));
    println!("|{}", "---|".repeat(columns.len() + 2));
    for group in groups {
        let rows = if group.too_few {
            vec![vec![TOO_FEW.to_string(); columns.len()]]
        } else {
            cells(group)
        };
        for row in rows {
            println!(
                "| {} | {} | {} |",
                group.name,
                group.sessions.len(),
                row.join(" | ")
            );
        }
    }
    println!();
}

fn efficiency(groups: &[Group], anonymous: bool) {
    let columns = [
        "issues",
        "projects",
        "models",
        "versions",
        "turns",
        "tool_calls",
        "output_tokens",
        "input_tokens",
        "validations",
        "other_messages",
    ];
    table("Efficiency", &columns, groups, |g| {
        let issues: HashSet<(&str, &str)> = g
            .sessions
            .iter()
            .flat_map(|r| {
                value(r, "issues")
                    .split(',')
                    .filter(|i| !i.is_empty())
                    .map(|i| (value(r, "project"), i))
            })
            .collect();
        let projects = if anonymous {
            g.sessions
                .iter()
                .map(|r| value(r, "project"))
                .collect::<HashSet<_>>()
                .len()
                .to_string()
        } else {
            mix(&g.sessions, "project")
        };
        let per_session =
            |f: &dyn Fn(&Row) -> u64| median(g.sessions.iter().map(|r| f(r)).collect()).to_string();
        let column = |c: &'static str| per_session(&move |r: &Row| number(r, c));
        vec![vec![
            issues.len().to_string(),
            projects,
            mix(&g.sessions, "models"),
            mix(&g.sessions, "versions"),
            column("turns"),
            column("tool_calls"),
            column("output_tokens"),
            per_session(&|r: &Row| {
                number(r, "input_tokens")
                    + number(r, "cache_read_tokens")
                    + number(r, "cache_creation_tokens")
            }),
            column("validations"),
            column("other_messages"),
        ]]
    });
}

/// The `name=n;name=n` pairs of `column` summed over `sessions`, as `name=n, …`, most frequent first, then by name.
fn summed_pairs(sessions: &[&Row], column: &str) -> String {
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for row in sessions {
        for (name, n) in value(row, column)
            .split(';')
            .filter_map(|p| p.split_once('='))
        {
            *counts.entry(name).or_default() += n.parse().unwrap_or(0);
        }
    }
    sorted_pairs(counts)
}

/// The sum of a column over `rows`, `None` when there is no row or one value is empty or not a number.
fn sum(rows: &[&Row], column: &str) -> Option<u64> {
    if rows.is_empty() {
        return None;
    }
    rows.iter()
        .map(|r| value(r, column).parse::<u64>().ok())
        .sum()
}

/// `a / b` with two decimals, empty when a term is missing or `b` is 0.
fn ratio(a: Option<u64>, b: Option<u64>) -> String {
    match (a, b) {
        (Some(a), Some(b)) if b > 0 => format!("{:.2}", a as f64 / b as f64),
        _ => String::new(),
    }
}

/// The rows of `inputs` whose `revision` belongs to `group`.
fn of_group<'a>(inputs: impl IntoIterator<Item = &'a Row>, group: &Group) -> Vec<&'a Row> {
    inputs
        .into_iter()
        .filter(|r| group.revisions.contains(value(r, "revision")))
        .collect()
}

fn quality(groups: &[Group], files: &[Vec<Row>]) {
    let summed = [
        "issues",
        "deliverables",
        "commits",
        "fixes_in_issue",
        "fixes_after",
        "reverts_after",
        "reviews",
        "review_findings",
        "traced",
        "traced_without_red",
        "sessions",
        "failed_checks",
        "validations",
        "other_messages",
    ];
    let mut columns: Vec<&str> = summed
        .iter()
        .map(|c| {
            if *c == "sessions" {
                "issue_sessions"
            } else {
                c
            }
        })
        .collect();
    columns.extend(["fixes_after_per_issue", "findings_per_review"]);
    table("Quality", &columns, groups, |g| {
        let rows = of_group(files.iter().flatten(), g);
        let total = |c: &str| sum(&rows, c);
        let mut cells: Vec<String> = summed
            .iter()
            .map(|c| total(c).map(|n| n.to_string()).unwrap_or_default())
            .collect();
        cells.push(ratio(total("fixes_after"), total("issues")));
        cells.push(ratio(total("review_findings"), total("reviews")));
        vec![cells]
    });
}

fn rules(groups: &[Group], breaches: &[Row]) {
    let columns = [
        "rule",
        "applied",
        "breached",
        "breaches",
        "blocked",
        "breached_share",
    ];
    table("Rules", &columns, groups, |g| {
        let mut by_rule: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
        for row in of_group(breaches, g) {
            by_rule.entry(value(row, "rule")).or_default().push(row);
        }
        if by_rule.is_empty() {
            return vec![vec![String::new(); columns.len()]];
        }
        by_rule
            .into_iter()
            .map(|(rule, rows)| {
                let total = |c: &str| rows.iter().map(|r| number(r, c)).sum::<u64>();
                let (applied, breached) = (total("applied"), total("breached"));
                let share = if applied > 0 {
                    format!("{}%", (breached * 200 + applied).div_euclid(applied * 2))
                } else {
                    String::new()
                };
                vec![
                    rule.to_string(),
                    applied.to_string(),
                    breached.to_string(),
                    total("breaches").to_string(),
                    total("blocked").to_string(),
                    share,
                ]
            })
            .collect()
    });
}

fn skills(groups: &[Group], context: Option<&[Row]>) {
    let mut columns = vec!["calls", "hook_refusals"];
    if context.is_some() {
        columns.push("unused");
    }
    table("Skills", &columns, groups, |g| {
        let mut cells = vec![
            summed_pairs(&g.sessions, "skills"),
            summed_pairs(&g.sessions, "hook_refusals"),
        ];
        if let Some(context) = context {
            let called: HashSet<&str> = g
                .sessions
                .iter()
                .flat_map(|r| value(r, "skills").split(';'))
                .filter_map(|p| p.split_once('='))
                .map(|(n, _)| n.rsplit(':').next().unwrap_or(n))
                .collect();
            let shipped: std::collections::BTreeSet<&str> = of_group(context, g)
                .iter()
                .flat_map(|r| value(r, "skills").split(','))
                .filter(|s| !s.is_empty())
                .collect();
            cells.push(
                shipped
                    .into_iter()
                    .filter(|s| !called.contains(s))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        vec![cells]
    });
}

fn fixed_context(groups: &[Group], context: &[Row]) {
    let bytes = [
        "claude_md_bytes",
        "agents_md_bytes",
        "skill_description_bytes",
    ];
    let mut columns = bytes.to_vec();
    columns.push("total_bytes");
    table("Fixed context", &columns, groups, |g| {
        let rows: Vec<&Row> = of_group(context, g)
            .into_iter()
            .filter(|r| bytes.iter().all(|c| value(r, c).parse::<u64>().is_ok()))
            .collect();
        let n = rows.len() as u64;
        let mean = |f: &dyn Fn(&Row) -> u64| {
            if n == 0 {
                String::new()
            } else {
                (rows.iter().map(|r| f(r)).sum::<u64>() * 2 + n)
                    .div_euclid(n * 2)
                    .to_string()
            }
        };
        let mut cells: Vec<String> = bytes
            .iter()
            .map(|c| mean(&|r: &Row| number(r, c)))
            .collect();
        cells.push(mean(&|r: &Row| bytes.iter().map(|c| number(r, c)).sum()));
        vec![cells]
    });
}

struct Options {
    sessions: Option<String>,
    breaches: Option<String>,
    quality: Vec<String>,
    context: Option<String>,
    min_sessions: usize,
    anonymous: bool,
}

fn options(args: &[String]) -> Result<Options, String> {
    let mut args = args.iter().cloned();
    let mut options = Options {
        sessions: None,
        breaches: None,
        quality: Vec::new(),
        context: None,
        min_sessions: 10,
        anonymous: false,
    };
    while let Some(arg) = args.next() {
        let mut file = || args.next().ok_or(format!("{arg} needs a file"));
        match arg.as_str() {
            "--sessions" => options.sessions = Some(file()?),
            "--breaches" => options.breaches = Some(file()?),
            "--quality" => options.quality.push(file()?),
            "--context" => options.context = Some(file()?),
            "--min-sessions" => {
                let n = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .filter(|n| (1..=1000).contains(n));
                options.min_sessions =
                    n.ok_or("--min-sessions needs a whole number from 1 to 1000")?;
            }
            "--anonymous" => options.anonymous = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

pub(super) fn main(args: &[String]) -> u8 {
    let fail = |e: String| -> ! {
        eprintln!("{e}\n{USAGE}");
        std::process::exit(2);
    };
    let options = options(args).unwrap_or_else(|e| fail(e));
    let sessions = options
        .sessions
        .as_deref()
        .ok_or("--sessions is required".to_string())
        .and_then(|f| read(f, SESSIONS))
        .unwrap_or_else(|e| fail(e));
    let breaches = options
        .breaches
        .as_deref()
        .map(|f| read(f, BREACHES))
        .transpose()
        .unwrap_or_else(|e| fail(e));
    let quality_files: Vec<Vec<Row>> = options
        .quality
        .iter()
        .map(|f| read(f, QUALITY))
        .collect::<Result<_, _>>()
        .unwrap_or_else(|e| fail(e));
    let context = options
        .context
        .as_deref()
        .map(|f| read(f, CONTEXT))
        .transpose()
        .unwrap_or_else(|e| fail(e));
    let groups = groups(&sessions, options.min_sessions);
    efficiency(&groups, options.anonymous);
    if !quality_files.is_empty() {
        quality(&groups, &quality_files);
    }
    if let Some(breaches) = &breaches {
        rules(&groups, breaches);
    }
    skills(&groups, context.as_deref());
    if let Some(context) = &context {
        fixed_context(&groups, context);
    }
    0
}
