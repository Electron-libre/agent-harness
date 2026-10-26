#!/usr/bin/env rust-script
//! Print the fixed context each harness revision costs a project in every session (`specs/harness-report.md`): one
//! tab-separated row per project and revision used by a session of the transcripts, with the bytes of `CLAUDE.md`,
//! `AGENTS.md` and the skill descriptions in the commit the revision came in force with, and the skills it ships.
//! Usage: `rust-script fixed_context.rs [projects_dir]` (default `~/.claude/projects`).
//! ```cargo
//! [dependencies]
//! serde_json = "1"
//! chrono = "0.4"
//! toml = "0.8"
//! ```

// transcripts.rs sha256: f7ba8cb58a3d105f7d88ef5445e1cb61658b6a70b9682d95966e9b88c327c03d
#[path = "transcripts.rs"]
mod transcripts;

use chrono::{DateTime, FixedOffset};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use transcripts::{default_branch, for_each_line, git, project_name, projects, projects_root, repository, revision, sessions, subagents, Place};

const HEADER: &str = "project\trevision\tcommit\tclaude_md_bytes\tagents_md_bytes\tskill_description_bytes\tskills";

/// The revision a commit's `.claude/harness.toml` pins, as EVA-8 names it; `None` for a `self:` revision, which is
/// matched by its own commit.
fn pinned(text: &str) -> Option<String> {
    let config: toml::Table = text.parse().ok()?;
    let field = |key: &str| config.get("harness").and_then(|h| h.get(key)).and_then(toml::Value::as_str);
    match (field("rev"), field("source")) {
        (Some(rev), _) => Some(rev.to_string()),
        (None, Some(".")) => None,
        (None, _) => Some("unpinned".to_string()),
    }
}

/// The oldest commit of the first-parent history of the default branch at which `revision` is in force (REP-2).
fn in_force_commit(repo: &Path, revision: &str) -> Option<String> {
    if let Some(commit) = revision.strip_prefix("self:") {
        return Some(commit.to_string());
    }
    let branch = default_branch(repo)?;
    let log = git(repo, &["log", "--first-parent", "--reverse", "--format=%H", branch, "--", ".claude/harness.toml"])?;
    log.lines().find(|c| git(repo, &["show", &format!("{c}:.claude/harness.toml")]).and_then(|t| pinned(&t)).is_some_and(|r| r == revision)).map(str::to_string)
}

/// The size of `path` in the tree of `commit`, 0 when absent.
fn size(repo: &Path, commit: &str, path: &str) -> u64 {
    git(repo, &["cat-file", "-s", &format!("{commit}:{path}")]).and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// The bytes of the top-level `name` and `description` values of a skill's front matter: surrounding quotes removed,
/// a value over several lines (a `>` or `|` block, or indented continuation lines) counted with one byte per line break.
fn description_bytes(skill: &str) -> u64 {
    let mut lines = skill.lines();
    if lines.next().map(str::trim) != Some("---") {
        return 0;
    }
    let mut values: Vec<Vec<&str>> = Vec::new();
    let mut counted = false;
    for line in lines.take_while(|l| l.trim() != "---") {
        if line.starts_with(char::is_whitespace) {
            if counted && !line.trim().is_empty() {
                values.last_mut().unwrap().push(line.trim());
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else { continue };
        counted = matches!(key, "name" | "description");
        if counted {
            let v = value.trim();
            let block = v.starts_with(['>', '|']);
            let unquoted = ['"', '\''].iter().find_map(|q| v.strip_prefix(*q).and_then(|s| s.strip_suffix(*q))).unwrap_or(v);
            values.push(if block || unquoted.is_empty() { Vec::new() } else { vec![unquoted] });
        }
    }
    values.iter().map(|parts| parts.iter().map(|p| p.len() as u64).sum::<u64>() + parts.len().saturating_sub(1) as u64).sum()
}

/// The columns after `revision` of a row: the commit and what its tree costs; empty when there is no such commit.
fn context(repo: &Path, revision: &str) -> Vec<String> {
    let Some(commit) = in_force_commit(repo, revision).and_then(|c| git(repo, &["rev-parse", "--short", &c])) else { return vec![String::new(); 5] };
    let files = git(repo, &["ls-tree", "-r", "--name-only", &commit, "--", ".claude/skills"]).unwrap_or_default();
    let skills: Vec<&str> = files.lines().filter_map(|p| p.strip_prefix(".claude/skills/")?.strip_suffix("/SKILL.md").filter(|d| !d.contains('/'))).collect();
    let descriptions: u64 = skills.iter().filter_map(|s| git(repo, &["show", &format!("{commit}:.claude/skills/{s}/SKILL.md")])).map(|t| description_bytes(&t)).sum();
    let (claude, agents) = (size(repo, &commit, "CLAUDE.md"), size(repo, &commit, "AGENTS.md"));
    vec![commit, claude.to_string(), agents.to_string(), descriptions.to_string(), skills.join(",")]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(flag) = args.iter().find(|a| a.starts_with('-')) {
        eprintln!("unknown option {flag}\nusage: fixed_context.rs [projects_dir]");
        std::process::exit(2);
    }
    let root = match projects_root(args.first()) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let mut repositories: HashMap<String, Option<PathBuf>> = HashMap::new();
    // (project, revision) -> (earliest start, repository)
    let mut used: BTreeMap<(String, String), (DateTime<FixedOffset>, PathBuf)> = BTreeMap::new();
    for project in projects(&root) {
        for path in sessions(&project) {
            let mut place = Place::default();
            for (file, main) in std::iter::once((path.clone(), true)).chain(subagents(&path).into_iter().map(|s| (s, false))) {
                if let Err(e) = for_each_line(&file, |line| {
                    place.observe(line, main);
                }) {
                    eprintln!("cannot read {}: {e}", file.display());
                }
            }
            let cwd = place.cwd.clone().unwrap_or_default();
            let repo = repositories.entry(cwd.clone()).or_insert_with(|| if cwd.is_empty() { None } else { repository(&cwd, &place.issues) });
            let (Some(repo), Some((start, text))) = (repo.clone(), &place.start) else { continue };
            let rev = revision(&repo, text);
            if rev.is_empty() {
                continue;
            }
            let entry = used.entry((project_name(&cwd, Some(&repo)), rev)).or_insert((*start, repo));
            entry.0 = entry.0.min(*start);
        }
    }
    let mut rows: Vec<_> = used.into_iter().collect();
    rows.sort_by(|((pa, _), (sa, _)), ((pb, _), (sb, _))| (pa, sa).cmp(&(pb, sb)));
    println!("{HEADER}");
    for ((project, rev), (_, repo)) in rows {
        println!("{project}\t{rev}\t{}", context(&repo, &rev).join("\t"));
    }
}
