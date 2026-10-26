#!/usr/bin/env rust-script
//! Behavior test for `harness_report.rs` on fixture TSV files: the groups of revisions, the tables per axis, the
//! confounders next to the numbers, the refusal of what it cannot read.
//! Usage: `just test-harness`.
//! ```cargo
//! [dependencies]
//! tempfile = "3"
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const SESSIONS: &str = "session\tproject\tissues\trevision\tmodels\tversions\tstart\tduration_s\tturns\ttool_calls\tinput_tokens\toutput_tokens\tthinking_tokens\tcache_read_tokens\tcache_creation_tokens\thook_refusals\tskills\tvalidations\tother_messages\tunreadable_lines\tunknown_lines\tmissing_fields";
const BREACHES: &str = "rule\trevision\tapplied\tbreached\tbreaches\tblocked";
const QUALITY: &str = "revision\tissues\tdeliverables\tcommits\tfixes_in_issue\tfixes_after\treverts_after\treviews\treview_findings\tsessions\tfailed_checks\tvalidations\tother_messages";
const CONTEXT: &str = "project\trevision\tcommit\tclaude_md_bytes\tagents_md_bytes\tskill_description_bytes\tskills";
const SELF: &str = "self:1111111111111111111111111111111111111111";

/// A session row: the columns that vary, the others fixed (input 1, output 10, cache read 2, cache creation 3).
struct S<'a> {
    project: &'a str,
    issues: &'a str,
    revision: &'a str,
    models: &'a str,
    day: u32,
    turns: u64,
    others: u64,
    hooks: &'a str,
    skills: &'a str,
}

impl S<'_> {
    fn row(&self, id: usize) -> String {
        let start = format!("2026-01-{:02}T00:00:{:02}.000Z", self.day, id % 60);
        let cells = [
            format!("s{id}"),
            self.project.into(),
            self.issues.into(),
            self.revision.into(),
            self.models.into(),
            "v1".into(),
            start,
            "60".into(),
            self.turns.to_string(),
            "2".into(),
            "1".into(),
            "10".into(),
            "0".into(),
            "2".into(),
            "3".into(),
            self.hooks.into(),
            self.skills.into(),
            "1".into(),
            self.others.to_string(),
            "0".into(),
            "0".into(),
            "0".into(),
        ];
        cells.join("\t")
    }
}

fn base<'a>(project: &'a str, revision: &'a str, day: u32, turns: u64) -> S<'a> {
    S { project, issues: "", revision, models: "m1", day, turns, others: 0, hooks: "", skills: "" }
}

/// The sessions: `SELF` 6 (p1, issue i1, turns 1 to 6), `r-b` 5 (3 of p2 on i1 with two models, 2 of p1; turns 7 to
/// 11), `r-c` 4 (turns 1 to 4, other messages 0, 0, 1, 1), and one with no revision, the earliest, left out. `r-c` is
/// written first: the order is the order of use.
fn sessions() -> Vec<S<'static>> {
    let mut list: Vec<S> = (1..=4).map(|t| S { others: u64::from(t > 2), ..base("p1", "r-c", 20, t) }).collect();
    list.push(base("p0", "", 1, 99));
    list.extend((1..=6).map(|t| S { issues: "i1", skills: "code-review=2", hooks: if t == 1 { "require_code_review=1" } else { "" }, ..base("p1", SELF, 2, t) }));
    list.extend((7..=9).map(|t| S { issues: "i1", models: "m1,m2", skills: "files:commit-message=1", ..base("p2", "r-b", 10, t) }));
    list.extend((10..=11).map(|t| base("p1", "r-b", 11, t)));
    list
}

fn write(path: &Path, header: &str, rows: &[String]) -> PathBuf {
    std::fs::write(path, std::iter::once(header.to_string()).chain(rows.iter().cloned()).map(|l| l + "\n").collect::<String>()).unwrap();
    path.to_path_buf()
}

fn run(dir: &Path, args: &[&std::ffi::OsStr]) -> (bool, String, String) {
    let out = Command::new("rust-script").arg(dir.join("harness_report.rs")).args(args).output().expect("rust-script must be installed");
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// The tables of a markdown report by heading: each a list of rows of trimmed cells, the header row first.
fn tables(out: &str) -> HashMap<String, Vec<Vec<String>>> {
    let mut tables: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let mut heading = String::new();
    for line in out.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            heading = h.trim().to_string();
        } else if line.starts_with('|') && !line.starts_with("|-") && !line.starts_with("| -") {
            let cells = line.trim().trim_matches('|').split('|').map(|c| c.trim().to_string()).collect();
            tables.entry(heading.clone()).or_default().push(cells);
        }
    }
    tables
}

/// The cell of `column` in the row of `group` of a table, empty when absent.
fn cell(table: Option<&Vec<Vec<String>>>, group: &str, column: &str) -> String {
    let Some(rows) = table else { return String::new() };
    let Some(index) = rows.first().and_then(|h| h.iter().position(|c| c == column)) else { return String::new() };
    rows.iter().skip(1).find(|r| r.first().is_some_and(|g| g == group)).and_then(|r| r.get(index)).cloned().unwrap_or_default()
}

// spec: REP-3, REP-4, REP-5, REP-8
fn efficiency_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let rows: Vec<String> = sessions().iter().enumerate().map(|(i, s)| s.row(i)).collect();
    let file = write(&tmp.join("sessions.tsv"), SESSIONS, &rows);
    let (ok, out, err) = run(dir, &["--sessions".as_ref(), file.as_os_str()]);
    if !ok {
        println!("{err}");
    }
    let t = tables(&out);
    let eff = t.get("Efficiency");
    let first = "self:1111111..r-b";
    let header: Vec<&str> = ["group", "sessions", "issues", "projects", "models", "versions", "turns", "tool_calls", "output_tokens", "input_tokens", "validations", "other_messages"].to_vec();
    let groups: Vec<String> = eff.map(|r| r.iter().skip(1).map(|c| c[0].clone()).collect()).unwrap_or_default();
    let too_few = eff.and_then(|r| r.iter().find(|c| c[0] == "r-c")).is_some_and(|c| c[1] == "4" && c[2..].iter().all(|v| v == "too few"));
    let (_, small, _) = run(dir, &["--sessions".as_ref(), file.as_os_str(), "--min-sessions".as_ref(), "2".as_ref()]);
    let small = tables(&small);
    let (_, anonymous, _) = run(dir, &["--sessions".as_ref(), file.as_os_str(), "--anonymous".as_ref()]);
    let anon = tables(&anonymous);
    vec![
        ("the report prints an efficiency table with its columns", ok && eff.and_then(|r| r.first()).is_some_and(|h| *h == header)),
        ("revisions in the order of use, grouped up to the threshold, the empty one left out", groups == [first, "r-c"]),
        ("a 40-digit revision is abbreviated to 7", out.contains("self:1111111") && !out.contains("self:11111111")),
        ("the last group under the threshold shows its size and `too few`", too_few),
        ("sessions, issues as project and issue pairs", cell(eff, first, "sessions") == "11" && cell(eff, first, "issues") == "2"),
        ("projects, models and versions by sessions, most frequent first", cell(eff, first, "projects") == "p1=8, p2=3" && cell(eff, first, "models") == "m1=11, m2=3" && cell(eff, first, "versions") == "v1=11"),
        ("medians per session of turns, tool calls and tokens", cell(eff, first, "turns") == "6" && cell(eff, first, "tool_calls") == "2" && cell(eff, first, "output_tokens") == "10" && cell(eff, first, "input_tokens") == "6"),
        ("an even count takes the mean of the middle values, rounded half up", cell(small.get("Efficiency"), "r-c", "turns") == "3" && cell(small.get("Efficiency"), "r-c", "other_messages") == "1"),
        ("--min-sessions sets the threshold", cell(small.get("Efficiency"), "self:1111111", "sessions") == "6" && cell(small.get("Efficiency"), "r-b", "sessions") == "5"),
        ("--anonymous gives the count of projects and no project name", cell(anon.get("Efficiency"), first, "projects") == "2" && !anonymous.contains("p1") && !anonymous.contains("p2")),
    ]
}

// spec: REP-6, REP-7
fn axes_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let rows: Vec<String> = sessions().iter().enumerate().map(|(i, s)| s.row(i)).collect();
    let sessions = write(&tmp.join("axes-sessions.tsv"), SESSIONS, &rows);
    let breaches = write(&tmp.join("breaches.tsv"), BREACHES, &[format!("x-rule\t{SELF}\t8\t1\t3\t0"), "x-rule\tr-b\t4\t1\t1\t2".into(), "a-rule\tr-b\t2\t0\t0\t0".into(), "x-rule\tr-c\t1\t1\t1\t0".into()]);
    let quality_a = write(&tmp.join("quality-a.tsv"), QUALITY, &[format!("{SELF}\t2\t4\t6\t1\t1\t0\t2\t3\t2\t1\t3\t4"), "zzz\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9".into()]);
    let quality_b = write(&tmp.join("quality-b.tsv"), QUALITY, &["r-b\t1\t1\t2\t0\t0\t0\t1\t\t1\t0\t1\t0".into()]);
    let context = write(
        &tmp.join("context.tsv"),
        CONTEXT,
        &[format!("p1\t{SELF}\tc1\t100\t200\t300\tcode-review,commit-message,write-spec"), "p1\tr-b\tc2\t201\t400\t500\taudit,code-review,commit-message".into(), "p2\tr-b\t\t\t\t\t".into()],
    );
    let arg = |s: &'static str| std::ffi::OsStr::new(s);
    let (ok, out, err) = run(dir, &[arg("--sessions"), sessions.as_os_str(), arg("--breaches"), breaches.as_os_str(), arg("--quality"), quality_a.as_os_str(), arg("--quality"), quality_b.as_os_str(), arg("--context"), context.as_os_str()]);
    if !ok {
        println!("{err}");
    }
    let t = tables(&out);
    let first = "self:1111111..r-b";
    let (quality, rules, skills, fixed) = (t.get("Quality"), t.get("Rules"), t.get("Skills"), t.get("Fixed context"));
    let rule_rows: Vec<String> = rules.map(|r| r.iter().skip(1).filter(|c| c[0] == first).map(|c| c[2..].join(" ")).collect()).unwrap_or_default();
    let few = |table: Option<&Vec<Vec<String>>>| table.is_some_and(|r| r.iter().filter(|c| c[0] == "r-c").count() == 1 && r.iter().any(|c| c[0] == "r-c" && c[1] == "4" && c[2..].iter().all(|v| v == "too few")));
    let too_few = [quality, rules, skills, fixed].into_iter().all(few);
    let (_, bare, _) = run(dir, &[arg("--sessions"), sessions.as_os_str()]);
    let bare = tables(&bare);
    vec![
        ("quality sums over the revisions and files of a group, other revisions left out", ok && cell(quality, first, "issues") == "3" && cell(quality, first, "commits") == "8" && cell(quality, first, "issue_sessions") == "3" && cell(quality, first, "other_messages") == "4"),
        ("an empty value empties its sum and its ratio", cell(quality, first, "review_findings").is_empty() && cell(quality, first, "findings_per_review").is_empty()),
        ("fixes after per issue with two decimals", cell(quality, first, "fixes_after_per_issue") == "0.33"),
        ("one rules row per rule of the group, sorted, sums and share", rule_rows == ["a-rule 2 0 0 0 0%", "x-rule 12 2 4 2 17%"]),
        ("skill calls and hook refusals summed, most frequent first", cell(skills, first, "calls") == "code-review=12, files:commit-message=3" && cell(skills, first, "hook_refusals") == "require_code_review=1"),
        ("unused skills of the revisions, a prefixed call counting", cell(skills, first, "unused") == "audit, write-spec"),
        ("fixed context means over the rows with bytes, rounded half up", cell(fixed, first, "claude_md_bytes") == "151" && cell(fixed, first, "agents_md_bytes") == "300" && cell(fixed, first, "skill_description_bytes") == "400" && cell(fixed, first, "total_bytes") == "851"),
        ("a group with too few sessions is one row of `too few` in every table", too_few),
        ("a table whose input is not given is not printed", bare.contains_key("Skills") && !bare.contains_key("Quality") && !bare.contains_key("Rules") && !bare.contains_key("Fixed context")),
    ]
}

// spec: REP-3
fn refusal_checks(dir: &Path, tmp: &Path) -> Vec<(&'static str, bool)> {
    let good = write(&tmp.join("good.tsv"), SESSIONS, &[base("p1", "r", 1, 1).row(0)]);
    let bad = write(&tmp.join("bad.tsv"), "session\tsomething_else", &[]);
    let refused = |args: &[&std::ffi::OsStr]| !run(dir, args).0;
    vec![
        ("a missing --sessions is refused", refused(&[])),
        ("a sessions file with another header is refused", refused(&["--sessions".as_ref(), bad.as_os_str()])),
        ("a quality file with another header is refused", refused(&["--sessions".as_ref(), good.as_os_str(), "--quality".as_ref(), bad.as_os_str()])),
        ("a --min-sessions out of range is refused", refused(&["--sessions".as_ref(), good.as_os_str(), "--min-sessions".as_ref(), "0".as_ref()])),
        ("an unknown option is refused", refused(&["--sessions".as_ref(), good.as_os_str(), "--bogus".as_ref()])),
        ("a readable input is accepted", run(dir, &["--sessions".as_ref(), good.as_os_str()]).0),
    ]
}

fn main() {
    let dir = std::fs::canonicalize(Path::new(file!()).parent().unwrap()).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let checks: Vec<_> = efficiency_checks(&dir, temp.path()).into_iter().chain(axes_checks(&dir, temp.path())).chain(refusal_checks(&dir, temp.path())).collect();
    let mut failed = false;
    for (name, ok) in checks {
        println!("{} - {name}", if ok { "ok  " } else { "FAIL" });
        failed |= !ok;
    }
    if failed {
        std::process::exit(1);
    }
}
