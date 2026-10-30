//! `release [<version> [--tag]]`: a numbered release of the project from its Conventional Commits
//! (`specs/release.md`). With no version, print the version proposed from the commits since the last `vX.Y.Z` tag.
//! With one, write its section of `CHANGELOG.md` and run the project's `release_version_command`, for the user to
//! reread and commit; with `--tag` too, once that commit is made, create the annotated tag `v<version>` on `HEAD`.
//! Never commits nor pushes: it prints the push command.

use std::fmt::{self, Write as _};
use std::path::Path;
use std::process::Command;

/// The header of a new `CHANGELOG.md`, the text of the seeded one.
pub(crate) const HEADER: &str = "# Changelog

The notable changes of each version, newest first ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
[Semantic Versioning](https://semver.org/)). Each section is written by `just release <version>` from the commits,
then reread before the release is committed.
";
const CHANGELOG: &str = "CHANGELOG.md";
/// The section of a release with no version tag before it.
const FIRST: &str = "First numbered release.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version(u64, u64, u64);

impl Version {
    /// `X.Y.Z`, an optional leading `v`.
    fn parse(text: &str) -> Option<Version> {
        let mut parts = text.strip_prefix('v').unwrap_or(text).split('.');
        let mut next = || {
            parts
                .next()
                .filter(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))?
                .parse()
                .ok()
        };
        let version = Version(next()?, next()?, next()?);
        parts.next().is_none().then_some(version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Where a commit goes in the changelog (REL-2, REL-4).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Breaking,
    Added,
    Fixed,
    Unlisted,
}

struct Commit {
    kind: Kind,
    description: String,
}

pub fn main(args: &[String]) -> u8 {
    match run(args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("release: {e}");
            1
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let root = super::repo_root()?;
    let root = Path::new(&root);
    let tag = args.iter().any(|a| a == "--tag");
    let versions: Vec<&String> = args.iter().filter(|a| *a != "--tag").collect();
    let version = match versions[..] {
        [] if !tag => {
            println!("{}", proposed(root)?.0);
            return Ok(());
        }
        [v] => Version::parse(v).ok_or_else(|| format!("`{v}` is not a version X.Y.Z"))?,
        _ => return Err("usage: release [<version> [--tag]]".into()),
    };
    if git(
        root,
        &[
            "rev-parse",
            "-q",
            "--verify",
            &format!("refs/tags/v{version}"),
        ],
    )
    .is_ok()
    {
        return Err(format!("the tag v{version} exists"));
    }
    if tag {
        tag_release(root, version)
    } else {
        prepare(root, version)
    }
}

/// Write the section of `version` and run the version command (REL-3 to REL-6).
fn prepare(root: &Path, version: Version) -> Result<(), String> {
    let (proposal, last, commits) = proposed(root)?;
    if last.is_some() && version < proposal {
        return Err(format!(
            "{version} is lower than {proposal}, the version the commits since v{} call for",
            last.unwrap_or(proposal)
        ));
    }
    let path = root.join(CHANGELOG);
    let before = std::fs::read_to_string(&path).ok();
    let text = before.clone().unwrap_or_else(|| HEADER.to_string());
    if section(&text, version).is_some() {
        return Err(format!("{CHANGELOG} already has a section for {version}"));
    }
    let today = chrono::Local::now().format("%Y-%m-%d");
    let mut new = format!("## [{version}] - {today}\n");
    if last.is_none() {
        let _ = write!(new, "\n{FIRST}\n");
    }
    for (kind, title) in [
        (Kind::Breaking, "Breaking"),
        (Kind::Added, "Added"),
        (Kind::Fixed, "Fixed"),
    ] {
        let lines: Vec<String> = commits
            .iter()
            .filter(|c| last.is_some() && c.kind == kind)
            .map(|c| format!("- {}\n", c.description))
            .collect();
        if !lines.is_empty() {
            let _ = write!(new, "\n### {title}\n\n{}", lines.concat());
        }
    }
    std::fs::write(&path, insert(&text, &new))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    if let Some(command) = project_str(root, "release_version_command") {
        let ok = Command::new("sh")
            .arg("-c")
            .arg(format!("{command} {version}"))
            .current_dir(root)
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            // Leave the changelog as it was, so the same release can be prepared again.
            let _ = match &before {
                Some(before) => std::fs::write(&path, before),
                None => std::fs::remove_file(&path),
            };
            return Err(format!(
                "the release_version_command `{command} {version}` failed"
            ));
        }
    }
    println!(
        "{CHANGELOG}: the section of {version} is written (proposed: {proposal}).\n\
         Next: reread and edit it, commit the release, then `just release {version} --tag`."
    );
    Ok(())
}

/// Tag `HEAD` with the committed section of `version` (REL-7, REL-10).
fn tag_release(root: &Path, version: Version) -> Result<(), String> {
    if git(root, &["diff", "--quiet", "HEAD"]).is_err() {
        return Err("the work tree has uncommitted changes: commit the release first".into());
    }
    let committed = git(root, &["show", &format!("HEAD:{CHANGELOG}")]).unwrap_or_default();
    let message = section(&committed, version)
        .ok_or_else(|| format!("the {CHANGELOG} of HEAD has no section for {version}"))?;
    let name = format!("v{version}");
    git(root, &["tag", "-a", &name, "-m", &message])?;
    println!("Tagged {name}. To publish it: git push origin {name}");
    Ok(())
}

/// The proposed version, the last version tag reachable from `HEAD`, and the commits since it (REL-1).
fn proposed(root: &Path) -> Result<(Version, Option<Version>, Vec<Commit>), String> {
    let last = git(root, &["tag", "--merged", "HEAD", "--list", "v*"])?
        .lines()
        .filter_map(Version::parse)
        .max();
    let commits = commits(root, last)?;
    let Version(major, minor, patch) = last.unwrap_or(Version(0, 0, 0));
    let proposal = match last {
        None => Version(0, 1, 0),
        Some(_) if commits.iter().any(|c| c.kind == Kind::Breaking) => Version(major + 1, 0, 0),
        Some(_) if commits.iter().any(|c| c.kind == Kind::Added) => Version(major, minor + 1, 0),
        Some(_) => Version(major, minor, patch + 1),
    };
    Ok((proposal, last, commits))
}

/// The non-merge commits since the tag of `last` (all of them without one), oldest first.
fn commits(root: &Path, last: Option<Version>) -> Result<Vec<Commit>, String> {
    let range = last.map_or("HEAD".to_string(), |v| format!("v{v}..HEAD"));
    let log = git(
        root,
        &[
            "log",
            "--reverse",
            "--no-merges",
            "--format=%x1e%H%x1f%B",
            &range,
        ],
    )?;
    let breaking_files = project_list(root, "release_breaking_files");
    let mut commits = Vec::new();
    for record in log.split('\u{1e}').filter(|r| !r.trim().is_empty()) {
        let (hash, message) = record.split_once('\u{1f}').unwrap_or((record, ""));
        let touches_listed = !breaking_files.is_empty()
            && git(
                root,
                &[
                    "diff-tree",
                    "--root",
                    "--no-commit-id",
                    "--name-only",
                    "-r",
                    hash.trim(),
                ],
            )?
            .lines()
            .any(|f| breaking_files.iter().any(|b| b == f));
        commits.push(classify(message, touches_listed));
    }
    Ok(commits)
}

/// The kind and description of a commit message; `touches_listed` when it changes a `release_breaking_files` file.
fn classify(message: &str, touches_listed: bool) -> Commit {
    let subject = message.lines().next().unwrap_or("").trim();
    let Some((head, description)) = subject.split_once(": ") else {
        return Commit {
            kind: Kind::Unlisted,
            description: subject.into(),
        };
    };
    let kind_of = head.split(['(', '!']).next().unwrap_or("");
    let footer = message
        .lines()
        .any(|l| l.starts_with("BREAKING CHANGE:") || l.starts_with("BREAKING-CHANGE:"));
    let kind = if head.ends_with('!') || footer || touches_listed {
        Kind::Breaking
    } else {
        match kind_of {
            "feat" => Kind::Added,
            "fix" => Kind::Fixed,
            _ => Kind::Unlisted,
        }
    };
    Commit {
        kind,
        description: description.trim().into(),
    }
}

/// The section of `version` in a changelog, from its heading to the next `## ` heading, trimmed.
fn section(text: &str, version: Version) -> Option<String> {
    let heading = format!("## [{version}]");
    let start = text.lines().position(|l| l.starts_with(&heading))?;
    let lines: Vec<&str> = text
        .lines()
        .skip(start)
        .enumerate()
        .take_while(|(i, l)| *i == 0 || !l.starts_with("## "))
        .map(|(_, l)| l)
        .collect();
    Some(lines.join("\n").trim().to_string())
}

/// `text` with `new` above its first `## ` section, or at its end when it has none.
fn insert(text: &str, new: &str) -> String {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with("## ") {
            return format!("{}{new}\n{}", &text[..offset], &text[offset..]);
        }
        offset += line.len();
    }
    let sep = if text.ends_with('\n') { "\n" } else { "\n\n" };
    format!("{text}{sep}{new}")
}

/// The value of `key` in the `[project]` table of `.claude/cardinal.toml`, if any.
fn project_value(root: &Path, key: &str) -> Option<toml::Value> {
    let text = std::fs::read_to_string(crate::config_of(root)).ok()?;
    let table: toml::Table = text.parse().ok()?;
    table.get("project")?.get(key).cloned()
}

fn project_str(root: &Path, key: &str) -> Option<String> {
    project_value(root, key)?
        .as_str()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(String::from)
}

fn project_list(root: &Path, key: &str) -> Vec<String> {
    project_value(root, key)
        .and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// The output of `git <args>` in `root`, or its error.
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seeded_changelog_is_the_header() {
        let seed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../seed/CHANGELOG.md");
        assert_eq!(std::fs::read_to_string(seed).unwrap(), HEADER);
    }
}
