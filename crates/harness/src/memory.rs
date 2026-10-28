//! The memory files a Claude Code session of a project loads at launch (`specs/harness-report.md`, REP-2), measured
//! in one way for a commit (`fixed-context`) and for the work tree (the budget, SYN-38).

use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Component, Path};

/// The files read in place of the `CLAUDE.md` files when neither exists, counted apart.
const AGENTS: [&str; 2] = ["AGENTS.md", ".claude/AGENTS.md"];

/// The hops of imports Claude Code follows from a memory file.
const HOPS: usize = 4;

/// A tree the memory files are read from; paths are relative and `/`-separated.
pub(crate) trait Tree {
    /// The content of the file at `path`, a symbolic link read as the file it points to; `None` when absent.
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    /// The paths of the files under the directory `dir`, at any depth.
    fn files(&self, dir: &str) -> Vec<String>;
}

/// The work tree at a path.
pub(crate) struct WorkTree<'a>(pub &'a Path);

impl Tree for WorkTree<'_> {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(self.0.join(path)).ok()
    }

    fn files(&self, dir: &str) -> Vec<String> {
        let mut files = Vec::new();
        let mut dirs = vec![dir.to_string()];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(self.0.join(&dir))
                .into_iter()
                .flatten()
                .flatten()
            {
                let path = format!("{dir}/{}", entry.file_name().to_string_lossy());
                // a linked directory is not walked: it could loop
                match entry.file_type() {
                    Ok(t) if t.is_dir() => dirs.push(path),
                    Ok(_) => files.push(path),
                    Err(_) => {}
                }
            }
        }
        files
    }
}

/// The bytes of the memory files loaded at launch: `AGENTS.md` and `.claude/AGENTS.md` apart from the others.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Memory {
    pub claude: u64,
    pub agents: u64,
}

/// Measures the memory files of `tree` (REP-2): each counted once, the imports followed breadth first so that a file
/// counts at its fewest hops.
pub(crate) fn measure(tree: &impl Tree) -> Memory {
    let mut roots: Vec<String> = ["CLAUDE.md", ".claude/CLAUDE.md"]
        .into_iter()
        .filter(|p| tree.read(p).is_some())
        .map(String::from)
        .collect();
    if roots.is_empty() {
        roots.extend(AGENTS.map(String::from));
    }
    let mut rules: Vec<String> = tree
        .files(".claude/rules")
        .into_iter()
        .filter(|p| p.ends_with(".md"))
        .collect();
    rules.sort();
    roots.extend(rules.into_iter().filter(|p| {
        tree.read(p)
            .is_some_and(|text| !scoped(&String::from_utf8_lossy(&text)))
    }));
    let mut memory = Memory::default();
    let mut seen = BTreeSet::new();
    let mut queue: VecDeque<(String, usize)> = roots.into_iter().map(|p| (p, 0)).collect();
    while let Some((path, hops)) = queue.pop_front() {
        if !seen.insert(path.clone()) {
            continue;
        }
        let Some(text) = tree.read(&path) else {
            continue;
        };
        let bytes = text.len() as u64;
        if AGENTS.contains(&path.as_str()) {
            memory.agents += bytes;
        } else {
            memory.claude += bytes;
        }
        if hops < HOPS {
            let dir = Path::new(&path).parent().unwrap_or(Path::new(""));
            queue.extend(
                imports(&String::from_utf8_lossy(&text))
                    .iter()
                    .filter_map(|i| resolve(dir, i))
                    .map(|p| (p, hops + 1)),
            );
        }
    }
    memory
}

/// Whether a rule's front matter has a top-level `paths` key: the rule loads on demand only.
fn scoped(rule: &str) -> bool {
    let mut lines = rule.lines();
    lines.next().map(str::trim) == Some("---")
        && lines
            .take_while(|l| l.trim() != "---")
            .any(|l| l.split_once(':').is_some_and(|(key, _)| key == "paths"))
}

/// The `@<path>` imports of a memory file: at the start of a line or after a white space, outside a code span and a
/// fenced code block, the path ending at the first white space a backslash does not escape; a quoted path imports
/// nothing.
fn imports(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut fence: Option<&str> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"]
            .into_iter()
            .find(|m| trimmed.starts_with(m) && line.len() - trimmed.len() < 4);
        match (fence, marker) {
            (None, Some(m)) => fence = Some(m),
            (Some(f), Some(m)) if f == m => fence = None,
            _ => {}
        }
        if fence.is_some() || marker.is_some() {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '`' {
                let run = chars[i..].iter().take_while(|&&c| c == '`').count();
                i += run;
                // the span ends at the next run of the same length; without one the backticks are literal
                if let Some(end) = (i..chars.len()).find(|&j| {
                    chars[j..].iter().take_while(|&&c| c == '`').count() == run
                        && (j == 0 || chars[j - 1] != '`')
                }) {
                    i = end + run;
                }
                continue;
            }
            if chars[i] == '@' && (i == 0 || chars[i - 1].is_whitespace()) {
                let mut path = String::new();
                i += 1;
                while i < chars.len() && !chars[i].is_whitespace() {
                    if chars[i] == '\\' && chars.get(i + 1) == Some(&' ') {
                        i += 1;
                    }
                    path.push(chars[i]);
                    i += 1;
                }
                if !path.is_empty() && !path.starts_with(['"', '\'']) {
                    found.push(path);
                }
                continue;
            }
            i += 1;
        }
    }
    found
}

/// The path of an import relative to the root of the tree; `None` for an absolute path, a path from `~`, or one
/// that leaves the tree.
fn resolve(dir: &Path, import: &str) -> Option<String> {
    if import.starts_with(['/', '~']) {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    for component in dir.join(import).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}
