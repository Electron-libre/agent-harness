//! Print the next issue to work on: the open issue of lowest `rank` in its front matter, with its title, its `why`
//! and the rest of the order (ADR 0019). Reports (stderr) the issues without a rank and the shared ranks. While
//! `issues/queue.md` exists (format 4), its lines give the order, and the issues missing from it and its lines
//! without a file are reported.
//! An issue is taken, and skipped, when a git worktree has a branch named after its stem checked out, or when such
//! a branch holds work that is not on the branch of the main worktree (patch-equivalent commits and a squash of any
//! number of commits count as on it: merging the branch would change no file). A branch with neither (left over after a merge, empty) is reported `Stale:` and takes nothing. The mark
//! comes from git alone, so every worktree sees it.
//! Usage: `just next-issue` (from the repo root, or pass the `issues` directory as argument); `--convert` turns
//! `issues/queue.md` into the ranks of the issue files and deletes it.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn title(dir: &Path, stem: &str) -> String {
    std::fs::read_to_string(dir.join(format!("{stem}.md")))
        .ok()
        .and_then(|t| {
            let body = split_front_matter(&t).map_or(0, |(_, _, body)| body);
            t[body..]
                .lines()
                .find_map(|l| l.strip_prefix("# ").map(str::to_string))
        })
        .unwrap_or_else(|| "(no file)".to_string())
}

/// `- [stem](stem.md) - why` gives `(stem, why)`; any other line (title, prose) is not an entry.
fn entry(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim().strip_prefix("- [")?;
    let (stem, rest) = rest.split_once("](")?;
    let why = rest
        .split_once(')')
        .map_or("", |(_, w)| w.trim_start_matches(" - "));
    Some((stem, why))
}

fn git_lines(dir: &Path, args: &[&str]) -> Vec<String> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `branch` holds work that `trunk` lacks; true when git cannot tell, so unmerged work is never ignored.
/// No work when every commit has a patch-equivalent on `trunk` (`git cherry`), or when merging the branch into `trunk`
/// would change no file (a squash of any number of commits): the merge tree is then the tree of `trunk`. Keep in sync
/// with `finish` of `issue_worktree.rs`.
fn has_own_commits(dir: &Path, trunk: &str, branch: &str) -> bool {
    let branch = format!("refs/heads/{branch}");
    let equivalent = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["cherry", trunk, &branch])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some_and(|o| {
            !String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.starts_with('+'))
        });
    if equivalent {
        return false;
    }
    let merged = git_lines(dir, &["merge-tree", "--write-tree", trunk, &branch])
        .first()
        .cloned();
    let trunk_tree = git_lines(dir, &["rev-parse", &format!("{trunk}^{{tree}}")])
        .first()
        .cloned();
    merged.is_none() || merged != trunk_tree
}

/// A local branch: the path of the worktree that has it checked out, if any, and whether it takes its issue.
struct Branch {
    name: String,
    worktree: Option<String>,
    taken: bool,
}

/// The branches of the repository that holds `dir`. The branch of the main worktree is left out: it is the trunk,
/// not an issue. Empty outside a git repository. Only the branches named after one of `stems` are examined for commits.
fn branches(dir: &Path, stems: &[&str]) -> Vec<Branch> {
    let mut out: Vec<(String, Option<String>)> = git_lines(
        dir,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )
    .into_iter()
    .map(|b| (b, None))
    .collect();
    let (mut path, mut first) = (String::new(), true);
    let mut trunk = String::new();
    for line in git_lines(dir, &["worktree", "list", "--porcelain"]) {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = p.to_string();
        } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
            if first {
                out.retain(|(name, _)| name != b);
                trunk = b.to_string();
            } else if let Some(entry) = out.iter_mut().find(|(name, _)| name == b) {
                entry.1 = Some(path.clone());
            }
            first = false;
        } else if let Some(h) = line
            .strip_prefix("HEAD ")
            .filter(|_| first && trunk.is_empty())
        {
            trunk = h.to_string();
        } else if line.is_empty() {
            first = false;
        }
    }
    out.into_iter()
        .map(|(name, worktree)| {
            let taken = worktree.is_some()
                || trunk.is_empty()
                || !stems.contains(&name.as_str())
                || has_own_commits(dir, &trunk, &name);
            Branch {
                name,
                worktree,
                taken,
            }
        })
        .collect()
}

/// The stems of the issue files of `dir` (`queue.md` excluded), sorted.
fn issue_stems(dir: &Path) -> Vec<String> {
    let mut stems: Vec<String> = std::fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "md"))
                .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .filter(|s| s != "queue")
                .collect()
        })
        .unwrap_or_default();
    stems.sort();
    stems
}

/// The order of a format 4 `queue.md` (NXT-4): its entries, and whether it drifted from the files.
fn from_queue(dir: &Path, queue: &str) -> (Vec<(String, String)>, bool) {
    let entries: Vec<(String, String)> = queue
        .lines()
        .filter_map(entry)
        .map(|(s, w)| (s.to_string(), w.to_string()))
        .collect();
    let mut drift = false;
    for (stem, _) in &entries {
        if !dir.join(format!("{stem}.md")).is_file() {
            eprintln!("queue line without a file: {stem}");
            drift = true;
        }
    }
    for s in issue_stems(dir) {
        if !entries.iter().any(|(e, _)| *e == s) {
            eprintln!("issue missing from the queue: {s}");
            drift = true;
        }
    }
    let ranked = issue_stems(dir).iter().any(|s| {
        std::fs::read_to_string(dir.join(format!("{s}.md")))
            .is_ok_and(|t| front_matter(&t, "rank").is_some())
    });
    if ranked {
        eprintln!("hint: issue files carry a `rank` that issues/queue.md hides: run `just next-issue --convert` in the main worktree (NXT-6)");
    }
    (entries, drift)
}

/// The lines of the front matter of `text` (between a first line `---` and the next `---`, line endings kept) and
/// the offsets of its closing line and of the body; `None` when the file opens no closed front matter.
fn split_front_matter(text: &str) -> Option<(Vec<&str>, usize, usize)> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end() != "---" {
        return None;
    }
    let (mut at, mut matter) = (first.len(), Vec::new());
    for l in lines {
        if l.trim_end() == "---" {
            return Some((matter, at, at + l.len()));
        }
        at += l.len();
        matter.push(l);
    }
    None
}

/// `value` without the single (a doubled `'` inside stands for one) or double quotes around it.
fn unquote(value: &str) -> String {
    let inner = |q: char| {
        (value.len() >= 2 && value.starts_with(q) && value.ends_with(q))
            .then(|| &value[1..value.len() - 1])
    };
    if let Some(v) = inner('\'') {
        v.replace("''", "'")
    } else {
        inner('"').unwrap_or(value).to_string()
    }
}

/// The value of `key` in the front matter of `text`, unquoted.
fn front_matter(text: &str, key: &str) -> Option<String> {
    let (matter, _, _) = split_front_matter(text)?;
    matter
        .iter()
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(':'))
        .map(|v| unquote(v.trim()))
}

/// The order of the `rank` of each issue file (NXT-1): ranked first, by rank then stem, then the unranked ones,
/// reported (NXT-2); a shared rank is reported without drift (NXT-3).
fn from_ranks(dir: &Path) -> (Vec<(String, String)>, bool) {
    let mut ranked: Vec<(u64, String, String)> = Vec::new();
    let mut unranked = Vec::new();
    for stem in issue_stems(dir) {
        let text = std::fs::read_to_string(dir.join(format!("{stem}.md"))).unwrap_or_default();
        let why = front_matter(&text, "why").unwrap_or_default();
        match front_matter(&text, "rank").and_then(|r| r.parse().ok()) {
            Some(rank) => ranked.push((rank, stem, why)),
            None => unranked.push((stem, why)),
        }
    }
    ranked.sort();
    for group in ranked.chunk_by(|a, b| a.0 == b.0).filter(|g| g.len() > 1) {
        let stems: Vec<&str> = group.iter().map(|(_, s, _)| s.as_str()).collect();
        eprintln!("same rank {}: {}", group[0].0, stems.join(", "));
    }
    for (stem, _) in &unranked {
        eprintln!("issue without a rank: {stem}");
    }
    let drift = !unranked.is_empty();
    let entries = ranked
        .into_iter()
        .map(|(_, s, w)| (s, w))
        .chain(unranked)
        .collect();
    (entries, drift)
}

/// `text` with `rank` and `why` (single-quoted; removed when empty) set in its front matter: an existing line of
/// either key is replaced in place with the line ending of the file, a missing one is added at the end of the front
/// matter, and a file without one gets it; nothing else changes.
fn with_rank(text: &str, rank: usize, why: &str) -> String {
    let mut set = vec![
        ("rank", Some(rank.to_string())),
        (
            "why",
            (!why.is_empty()).then(|| format!("'{}'", why.replace('\'', "''"))),
        ),
    ];
    let Some((matter, closing, _)) = split_front_matter(text) else {
        let head: String = set
            .iter()
            .filter_map(|(k, v)| Some(format!("{k}: {}\n", v.as_ref()?)))
            .collect();
        return format!("---\n{head}---\n{text}");
    };
    let eol = if text.starts_with("---\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut out = text[..text.find('\n').map_or(0, |i| i + 1)].to_string();
    for l in matter {
        let key = set
            .iter()
            .position(|(k, _)| l.strip_prefix(k).is_some_and(|r| r.starts_with(':')));
        if let Some(i) = key {
            if let (k, Some(v)) = set.remove(i) {
                let _ = write!(out, "{k}: {v}{eol}");
            }
        } else {
            out.push_str(l);
        }
    }
    for (k, v) in set {
        if let Some(v) = v {
            let _ = write!(out, "{k}: {v}{eol}");
        }
    }
    out + &text[closing..]
}

/// `--convert` (NXT-5): the rank of each line of `queue.md` (100 times its position) and its reason go to the front
/// matter of its issue file, then `queue.md` is deleted. Exit 1 when a line had no file.
fn convert(dir: &Path) -> u8 {
    if let Some((top, main)) = super::close_issue::linked_worktree(dir) {
        eprintln!(
            "{} is a linked worktree: convert the queue in the main worktree {main}",
            top.display()
        );
        return 1;
    }
    let path = dir.join("queue.md");
    let Ok(queue) = std::fs::read_to_string(&path) else {
        eprintln!("no {}", path.display());
        return 2;
    };
    let mut drift = false;
    for (i, (stem, why)) in queue.lines().filter_map(entry).enumerate() {
        let file = dir.join(format!("{stem}.md"));
        let Ok(text) = std::fs::read_to_string(&file) else {
            eprintln!("queue line without a file: {stem}");
            drift = true;
            continue;
        };
        let rank = 100 * (i + 1);
        std::fs::write(&file, with_rank(&text, rank, why)).expect("issue file is writable");
        println!("{stem}: rank {rank}");
    }
    std::fs::remove_file(&path).expect("queue is removable");
    println!("removed {}", path.display());
    u8::from(drift)
}

pub(super) fn main(args: &[String]) -> u8 {
    let convert_flag = args.iter().any(|a| a == "--convert");
    let dir = args
        .iter()
        .find(|a| *a != "--convert")
        .cloned()
        .unwrap_or_else(|| "issues".to_string());
    let dir = Path::new(&dir);
    if convert_flag {
        return convert(dir);
    }
    if !dir.is_dir() {
        eprintln!("no {} directory", dir.display());
        std::process::exit(2);
    }
    let (entries, drift) = match std::fs::read_to_string(dir.join("queue.md")) {
        Ok(queue) => from_queue(dir, &queue),
        Err(_) => from_ranks(dir),
    };
    let entries: Vec<(&str, &str)> = entries
        .iter()
        .map(|(s, w)| (s.as_str(), w.as_str()))
        .collect();

    let stems: Vec<&str> = entries.iter().map(|(s, _)| *s).collect();
    let branches = branches(dir, &stems);
    let taken = |stem: &str| {
        branches
            .iter()
            .find(|b| b.name == stem && b.taken)
            .map(|b| b.worktree.as_deref())
    };
    let stale = |stem: &str| branches.iter().any(|b| b.name == stem && !b.taken);
    let free: Vec<(usize, &(&str, &str))> = entries
        .iter()
        .enumerate()
        .filter(|(_, (s, _))| taken(s).is_none())
        .collect();

    match free.first() {
        None if entries.is_empty() => println!("No open issue."),
        None => println!("Every open issue is taken."),
        Some((_, (stem, why))) => {
            println!(
                "Next: issues/{stem}.md\n  {}\n  why here: {why}",
                title(dir, stem)
            );
            for (i, (s, _)) in free.iter().skip(1) {
                println!("{:>3}. {s}", i + 1);
            }
        }
    }
    for (stem, _) in &entries {
        if let Some(path) = taken(stem) {
            println!(
                "Taken: {stem}{}",
                path.map_or(String::new(), |p| format!(" (worktree {p})"))
            );
        }
    }
    for (stem, _) in &entries {
        if stale(stem) {
            println!("Stale: {stem} (branch with no worktree and nothing off the main branch; run `just finish-issue {stem}`)");
        }
    }
    std::process::exit(i32::from(drift));
}
