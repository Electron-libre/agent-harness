//! Print the fixed context each harness revision costs a project in every session (`specs/harness-report.md`): one
//! tab-separated row per project and revision used by a session of the transcripts, with the bytes of `CLAUDE.md`,
//! `AGENTS.md` and the skill descriptions in the commit the revision came in force with, and the skills it ships.
//! Usage: `harness run fixed-context [projects_dir]` (default `~/.claude/projects`).

use super::transcripts;

use chrono::{DateTime, FixedOffset};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use transcripts::{
    default_branch, for_each_line, git, pinned_revision, project_name, projects, projects_root,
    repository, revision, sessions, subagents, Place,
};

const HEADER: &str =
    "project\trevision\tcommit\tclaude_md_bytes\tagents_md_bytes\tskill_description_bytes\tskills";

/// The commits of the first-parent history of the default branch touching `.claude/harness.toml`, oldest first,
/// with the revision each pins (`None` for a `self:` revision, matched by its own commit).
fn pins(repo: &Path) -> Vec<(String, Option<String>)> {
    let Some(branch) = default_branch(repo) else {
        return Vec::new();
    };
    let log = git(
        repo,
        &[
            "log",
            "--first-parent",
            "--reverse",
            "--format=%H",
            branch,
            "--",
            ".claude/harness.toml",
        ],
    )
    .unwrap_or_default();
    log.lines()
        .filter_map(|c| {
            let config: toml::Table = git(repo, &["show", &format!("{c}:.claude/harness.toml")])?
                .parse()
                .ok()?;
            Some((c.to_string(), pinned_revision(&config)))
        })
        .collect()
}

/// The oldest commit at which `revision` is in force (REP-2), among the `pins` of its repository.
fn in_force_commit(pins: &[(String, Option<String>)], revision: &str) -> Option<String> {
    if let Some(commit) = revision.strip_prefix("self:") {
        return Some(commit.to_string());
    }
    pins.iter()
        .find(|(_, r)| r.as_deref() == Some(revision))
        .map(|(c, _)| c.clone())
}

/// The size of `path` in the tree of `commit`, a symbolic link counted as the file it points to; 0 when absent.
fn size(repo: &Path, commit: &str, path: &str) -> u64 {
    let link = git(repo, &["ls-tree", commit, "--", path])
        .is_some_and(|entry| entry.starts_with("120000 "));
    let target = if link {
        git(repo, &["show", &format!("{commit}:{path}")]).map(|t| {
            Path::new(path)
                .with_file_name(t)
                .to_string_lossy()
                .into_owned()
        })
    } else {
        Some(path.to_string())
    };
    target
        .and_then(|t| git(repo, &["cat-file", "-s", &format!("{commit}:{t}")]))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// The bytes of the top-level `name` and `description` values of a skill's front matter: surrounding quotes removed,
/// a value over several lines (a `>` or `|` block, or continuation lines), blank lines included, counted with one byte
/// per line break.
pub(crate) fn description_bytes(skill: &str) -> u64 {
    let mut lines = skill.lines();
    if lines.next().map(str::trim) != Some("---") {
        return 0;
    }
    let mut values: Vec<Vec<&str>> = Vec::new();
    let mut counted = false;
    for line in lines.take_while(|l| l.trim() != "---") {
        if line.trim().is_empty() || line.starts_with(char::is_whitespace) {
            if counted {
                values.last_mut().unwrap().push(line.trim());
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        counted = matches!(key, "name" | "description");
        if counted {
            let v = value.trim();
            values.push(if v.is_empty() || v.starts_with(['>', '|']) {
                Vec::new()
            } else {
                vec![v]
            });
        }
    }
    values
        .iter_mut()
        .map(|parts| {
            while parts.last().is_some_and(|p| p.is_empty()) {
                parts.pop();
            }
            let (first, last) = (
                parts.first().copied().unwrap_or_default(),
                parts.last().copied().unwrap_or_default(),
            );
            let quoted = ['"', '\'']
                .into_iter()
                .any(|q| first.starts_with(q) && last.ends_with(q))
                && (parts.len() > 1 || first.len() > 1);
            if quoted {
                let n = parts.len() - 1;
                parts[0] = &parts[0][1..];
                parts[n] = &parts[n][..parts[n].len() - 1];
            }
            parts.iter().map(|p| p.len() as u64).sum::<u64>() + parts.len().saturating_sub(1) as u64
        })
        .sum()
}

/// The columns after `revision` of a row: the commit and what its tree costs; empty when there is no such commit.
fn context(repo: &Path, pins: &[(String, Option<String>)], revision: &str) -> Vec<String> {
    let Some(commit) =
        in_force_commit(pins, revision).and_then(|c| git(repo, &["rev-parse", "--short", &c]))
    else {
        return vec![String::new(); 5];
    };
    let files = git(
        repo,
        &[
            "-c",
            "core.quotePath=false",
            "ls-tree",
            "-r",
            "--name-only",
            &commit,
            "--",
            ".claude/skills",
        ],
    )
    .unwrap_or_default();
    let mut skills: Vec<&str> = files
        .lines()
        .filter_map(|p| {
            p.strip_prefix(".claude/skills/")?
                .strip_suffix("/SKILL.md")
                .filter(|d| !d.contains('/'))
        })
        .collect();
    skills.sort_unstable();
    let descriptions: u64 = skills
        .iter()
        .filter_map(|s| {
            git(
                repo,
                &["show", &format!("{commit}:.claude/skills/{s}/SKILL.md")],
            )
        })
        .map(|t| description_bytes(&t))
        .sum();
    let (claude, agents) = (
        size(repo, &commit, "CLAUDE.md"),
        size(repo, &commit, "AGENTS.md"),
    );
    vec![
        commit,
        claude.to_string(),
        agents.to_string(),
        descriptions.to_string(),
        skills.join(","),
    ]
}

pub(super) fn main(args: &[String]) -> u8 {
    let args: Vec<String> = args.to_vec();
    if let Some(flag) = args.iter().find(|a| a.starts_with('-')) {
        eprintln!("unknown option {flag}\nusage: harness run fixed-context [projects_dir]");
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
            for (file, main) in std::iter::once((path.clone(), true))
                .chain(subagents(&path).into_iter().map(|s| (s, false)))
            {
                if let Err(e) = for_each_line(&file, |line| {
                    place.observe(line, main);
                }) {
                    eprintln!("cannot read {}: {e}", file.display());
                }
            }
            let cwd = place.cwd.clone().unwrap_or_default();
            let repo = repositories.entry(cwd.clone()).or_insert_with(|| {
                if cwd.is_empty() {
                    None
                } else {
                    repository(&cwd, &place.issues)
                }
            });
            let (Some(repo), Some((start, text))) = (repo.clone(), &place.start) else {
                continue;
            };
            let rev = revision(&repo, text);
            if rev.is_empty() {
                continue;
            }
            let entry = used
                .entry((project_name(&cwd, Some(&repo)), rev))
                .or_insert((*start, repo));
            entry.0 = entry.0.min(*start);
        }
    }
    let mut rows: Vec<_> = used.into_iter().collect();
    rows.sort_by(|((pa, _), (sa, _)), ((pb, _), (sb, _))| (pa, sa).cmp(&(pb, sb)));
    let mut pinned: HashMap<PathBuf, Vec<(String, Option<String>)>> = HashMap::new();
    println!("{HEADER}");
    for ((project, rev), (_, repo)) in rows {
        let pins = pinned.entry(repo.clone()).or_insert_with(|| pins(&repo));
        println!(
            "{project}\t{rev}\t{}",
            context(&repo, pins, &rev).join("\t")
        );
    }
    0
}
