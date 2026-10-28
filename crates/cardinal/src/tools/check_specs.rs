//! Validate `specs/*.md` against `specs/README.md` and trace them to the tests (`// spec: ID`).
//! Usage: `just check-specs` (from the repo root; an optional argument is another root; `--strict` also flags the
//! citations of managed tests (under `.claude/`) whose spec the project lacks: for the repository that ships them).
//! Prints one line per problem, exit 1 if any; a `[NEEDS CLARIFICATION: ...]` marker is a problem in an `accepted`
//! spec and a warning on stderr in a `draft` one; the vague wording of a `draft` requirement (`VAGUE`, not exactly one
//! `must`) is a warning, and so are the requirements proven only by tests under a `src` directory whose citations
//! give no reason, `// spec: ID (internal: <reason>)` (TRC-1, TRC-6).

use super::repo_files;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const SECTIONS: [&str; 4] = [
    "## Intent",
    "## Requirements",
    "## Scenarios",
    "## Non-goals",
];

pub(super) struct Requirement {
    pub(super) id: String,
    /// Needs no test: withdrawn, or of the former `quality` kind.
    pub(super) exempt: bool,
}

pub(super) struct Spec {
    file: String,
    id: String,
    pub(super) status: String,
    pub(super) requirements: Vec<Requirement>,
}

fn is_id_prefix(s: &str) -> bool {
    s.len() == 3 && s.chars().all(|c| c.is_ascii_uppercase())
}

/// Whether `s` is a requirement id (`ABC-3`); `delivered-quality` reads the citations with it.
pub(super) fn is_req_id(s: &str) -> bool {
    s.split_once('-').is_some_and(|(p, n)| {
        is_id_prefix(p) && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())
    })
}

/// `line` with the content of its inline code spans (a run of backticks up to the next run of the same length)
/// blanked out, so a cited marker or word is not read as the spec's own.
fn without_code_spans(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        out.push_str(&rest[..start]);
        let run = rest[start..].len() - rest[start..].trim_start_matches('`').len();
        let after = &rest[start + run..];
        let fence = "`".repeat(run);
        let close = after
            .match_indices(&fence)
            .find(|(i, _)| !after[i + run..].starts_with('`') && !after[..*i].ends_with('`'));
        let Some((end, _)) = close else {
            out.push_str(&rest[start..]);
            return out;
        };
        out.push(' ');
        rest = &after[end + run..];
    }
    out.push_str(rest);
    out
}

const MARKER: &str = "[NEEDS CLARIFICATION:";

/// The text of a marker up to its closing bracket, brackets inside it included.
fn question(after: &str) -> &str {
    let mut depth = 1;
    for (i, c) in after.char_indices() {
        depth += match c {
            '[' => 1,
            ']' => -1,
            _ => 0,
        };
        if depth == 0 {
            return after[..i].trim();
        }
    }
    after.trim()
}

/// An open question left in the spec: a problem once `accepted`, a warning while `draft`. A marker cited in a code
/// span or a fenced code block is an example, not a question.
fn check_markers(
    file: &str,
    lines: &[&str],
    accepted: bool,
    out: &mut Vec<String>,
    warn: &mut Vec<String>,
) {
    for (i, (line, fenced)) in super::fenced(lines.iter().copied()).enumerate() {
        if fenced {
            continue;
        }
        let text = without_code_spans(line);
        let mut rest = text.as_str();
        while let Some(start) = rest.find(MARKER) {
            let after = &rest[start + MARKER.len()..];
            let question = question(after);
            if accepted {
                out.push(format!(
                    "{file}:{}: `{MARKER} ...]` left in an accepted spec: settle it",
                    i + 1
                ));
            } else {
                warn.push(format!(
                    "warning: {file}:{}: needs clarification: {question}",
                    i + 1
                ));
            }
            rest = after;
        }
    }
}

/// Words that leave the obligation of a requirement open (`etc` stands for `etc.`).
const VAGUE: [&str; 5] = ["should", "may", "etc", "appropriate", "fast"];

/// `text` without its double-quoted parts, straight or curly: a quoted message is not the requirement's wording. A
/// quote with no closing one (an inch mark) is kept, so the text after it is still checked.
fn without_quotes(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some((start, open)) = rest
        .char_indices()
        .find(|(_, c)| matches!(c, '"' | '\u{201c}'))
    {
        let after = &rest[start + open.len_utf8()..];
        let close = if open == '"' { '"' } else { '\u{201d}' };
        out.push_str(&rest[..start]);
        match after.find(close) {
            Some(end) => rest = &after[end + close.len_utf8()..],
            None => {
                out.push(open);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Warns of a `draft` requirement whose wording is vague: a word of `VAGUE`, or not exactly one `must`. Warnings
/// only: the word list has false positives.
fn check_wording(file: &str, id: &str, statement: &str, warn: &mut Vec<String>) {
    // `shouldn't` holds `should` and `mustn't` a `must`
    let text = without_quotes(&without_code_spans(statement))
        .to_lowercase()
        .replace("n't", " not")
        .replace("n\u{2019}t", " not");
    let words: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let mut warned = BTreeSet::new();
    for word in words.iter().filter(|w| VAGUE.contains(w)) {
        if warned.insert(*word) {
            let shown = if *word == "etc" { "etc." } else { word };
            warn.push(format!(
                "warning: {file}: requirement {id}: vague word `{shown}`"
            ));
        }
    }
    let musts = words.iter().filter(|w| **w == "must").count();
    if musts != 1 {
        warn.push(format!(
            "warning: {file}: requirement {id}: holds {musts} `must`, expected 1"
        ));
    }
}

fn parse(
    root: &Path,
    file: &str,
    text: &str,
    out: &mut Vec<String>,
    warn: &mut Vec<String>,
) -> Spec {
    let mut problem = |m: String| out.push(format!("{file}: {m}"));
    let mut spec = Spec {
        file: file.to_string(),
        id: String::new(),
        status: String::new(),
        requirements: Vec::new(),
    };
    let lines: Vec<&str> = text.lines().collect();
    let close = lines
        .iter()
        .skip(1)
        .position(|l| l.trim() == "---")
        .map(|p| p + 1);
    if lines.first().map(|l| l.trim()) != Some("---") || close.is_none() {
        problem("no front matter (`---` ... `---`)".to_string());
        return spec;
    }
    let close = close.unwrap_or(0);
    for line in &lines[1..close] {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.split('#').next().unwrap_or("").trim();
        match key.trim() {
            "id" => spec.id = value.to_string(),
            "status" => spec.status = value.to_string(),
            "adrs" => {
                for n in value
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                {
                    let found = std::fs::read_dir(root.join("docs/adr"))
                        .into_iter()
                        .flatten()
                        .flatten()
                        .any(|e| {
                            e.file_name()
                                .to_string_lossy()
                                .starts_with(&format!("{n}-"))
                        });
                    if !found {
                        problem(format!("adr {n} not found in docs/adr"));
                    }
                }
            }
            _ => {}
        }
    }
    if !is_id_prefix(&spec.id) {
        problem(format!(
            "front matter `id` must be three capital letters, got `{}`",
            spec.id
        ));
    }
    // `implemented` was a status of its own, enforced like `accepted`: still read as such so older specs keep passing
    if spec.status == "implemented" {
        warn.push(format!(
            "warning: {file}: status `implemented` is deprecated, use `accepted`"
        ));
        spec.status = "accepted".to_string();
    }
    if !["draft", "accepted"].contains(&spec.status.as_str()) {
        problem(format!(
            "front matter `status` must be draft or accepted, got `{}`",
            spec.status
        ));
    }
    for section in SECTIONS {
        if !lines.iter().any(|l| l.trim() == section) {
            problem(format!("missing section `{section}`"));
        }
    }
    let mut seen = BTreeSet::new();
    let mut in_requirements = false;
    for line in &lines {
        if line.starts_with("## ") {
            in_requirements = line.trim() == "## Requirements";
        }
        if !in_requirements || !line.starts_with("- ") {
            continue;
        }
        let parsed = line
            .strip_prefix("- **")
            .and_then(|r| r.split_once("**"))
            .filter(|(id, _)| is_req_id(id));
        let Some((id, after)) = parsed else {
            problem(format!(
                "requirement line must start with `- **{}-<n>**`: `{line}`",
                spec.id
            ));
            continue;
        };
        if !id.starts_with(&format!("{}-", spec.id)) {
            problem(format!(
                "requirement {id} does not start with the spec id `{}`",
                spec.id
            ));
        }
        if !seen.insert(id.to_string()) {
            problem(format!("duplicate requirement {id}"));
        }
        let kind = after
            .trim()
            .strip_prefix('(')
            .and_then(|a| a.split_once(')'));
        let (kind, statement) = kind.map_or((String::new(), ""), |(k, rest)| {
            (k.to_string(), rest.trim())
        });
        let withdrawn = statement.starts_with("withdrawn:");
        // `quality` was a kind proved by a benchmark: still read, untraced, so older specs keep passing
        if kind == "quality" && !withdrawn {
            warn.push(format!("warning: {file}: requirement {id}: kind `quality` is deprecated, reword it for a test or withdraw it"));
        } else if kind != "deterministic" && kind != "quality" {
            problem(format!(
                "requirement {id}: kind must be `(deterministic)`, got `({kind})`"
            ));
        }
        // with no `(kind)`, the statement was not found: the format problem is enough
        if spec.status == "draft" && !withdrawn && !kind.is_empty() {
            check_wording(file, id, statement, warn);
        }
        spec.requirements.push(Requirement {
            id: id.to_string(),
            exempt: withdrawn || kind == "quality",
        });
    }
    check_markers(file, &lines, spec.status == "accepted", out, warn);
    spec
}

/// Every file under `dir` (outside a git repository, where no worktree can nest), but those of the directories
/// `skipped` says to leave; a symbolic link is never followed, as in `repo_files` (ADR 0009).
fn all_files(dir: &Path, skipped: &dyn Fn(&Path) -> bool, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() && !skipped(&path) {
            all_files(&path, skipped, found);
        } else if kind.is_file() {
            found.push(path);
        }
    }
}

/// The ids of a `// spec:` list and the reason it gives for an internal proof, `A-1, B-2 (internal: <reason>)`
/// (TRC-6, TRC-8): the reason never adds an id. The reason is the trimmed text after `(internal:`, its closing `)`
/// included.
pub(super) fn citation(list: &str) -> (impl Iterator<Item = &str>, Option<&str>) {
    let (ids, reason) = match list.split_once("(internal:") {
        Some((ids, reason)) => (ids, Some(reason.trim())),
        None => (list, None),
    };
    let ids = ids
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty());
    (ids, reason)
}

/// The `.rs` files of the root that may hold tests, whatever the layout of the project (TRC-9): all but those under
/// `.git`, `node_modules`, a `target` beside a `Cargo.toml` (the build output) or a directory named like one of the
/// `skipped_dirs` of `.claude/harness.toml` (TRC-10); a failure of git, or a `harness.toml` that does not parse, is a
/// problem in `out`.
pub(super) fn test_files(root: &Path, out: &mut Vec<String>) -> Vec<PathBuf> {
    let mut names = vec![".git".to_string(), "node_modules".to_string()];
    match super::config_list(root, "skipped_dirs") {
        Ok(dirs) => names.extend(dirs),
        // a project with no `harness.toml` skips nothing more
        Err(e) if crate::config_of(root).exists() => out.push(e),
        Err(_) => {}
    }
    let skipped = |dir: &Path| {
        dir.file_name().is_some_and(|name| {
            names.iter().any(|n| name == n.as_str())
                || (name == "target" && dir.with_file_name("Cargo.toml").is_file())
        })
    };
    // in a git repository the files git lists, so never those of a nested worktree (ADR 0008) nor the ignored ones
    // git's `*` also matches `/`: the `.rs` files at any depth
    let files: Vec<PathBuf> = match repo_files::repo_files(root, &["*.rs"]) {
        Ok(Some(files)) => files
            .into_iter()
            .filter(|f| {
                let dirs = f.strip_prefix(root).unwrap_or(f).ancestors().skip(1);
                !dirs
                    .filter(|dir| !dir.as_os_str().is_empty())
                    .any(|dir| skipped(&root.join(dir)))
            })
            .collect(),
        Ok(None) => {
            let mut files = Vec::new();
            all_files(root, &skipped, &mut files);
            files
        }
        // never fall back to the disk, which holds the nested worktrees
        Err(e) => {
            out.push(e);
            Vec::new()
        }
    };
    files
        .into_iter()
        .filter(|f| f.extension().is_some_and(|e| e == "rs"))
        .collect()
}

/// A file citing a requirement, and whether that citation gives the reason of an internal proof.
pub(super) struct Cite {
    pub(super) file: String,
    reasoned: bool,
}

/// Ids cited by the `// spec: A-1, B-2` comments of `files`, with the files that cite them and whether the citation gives a
/// reason. A citation sits right above a test: only attributes and other comments may come between it and the `fn`;
/// any other place is a problem, and so is a reason that is blank, misplaced or unclosed (TRC-7).
pub(super) fn citations(
    root: &Path,
    files: &[PathBuf],
    out: &mut Vec<String>,
) -> BTreeMap<String, Vec<Cite>> {
    let mut cited: BTreeMap<String, Vec<Cite>> = BTreeMap::new();
    for file in files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        let name = file
            .strip_prefix(root)
            .unwrap_or(file)
            .display()
            .to_string();
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let Some(list) = line.trim().strip_prefix("// spec:") else {
                continue;
            };
            let next = lines[i + 1..]
                .iter()
                .map(|l| l.trim())
                .find(|l| !l.starts_with("#[") && !l.starts_with("//"));
            if !next.is_some_and(|l| l.contains("fn ")) {
                out.push(format!(
                    "{name}:{}: `// spec:` must sit right above a test `fn`",
                    i + 1
                ));
            }
            let (ids, reason) = citation(list);
            let ids: Vec<&str> = ids.collect();
            let problem = reason.and_then(|r| match r.strip_suffix(')') {
                // a `)` of the reason closes one of its own `(`: an unmatched one ends the reason early
                Some(r) if !ids.is_empty() && r.matches('(').count() == r.matches(')').count() => {
                    r.trim().is_empty().then_some("gives no reason")
                }
                _ => Some("must follow the ids and end the line"),
            });
            if let Some(problem) = problem {
                out.push(format!(
                    "{name}:{}: `(internal: <reason>)` {problem}",
                    i + 1
                ));
            }
            let reasoned = reason.is_some() && problem.is_none();
            for id in ids {
                cited.entry(id.to_string()).or_default().push(Cite {
                    file: name.clone(),
                    reasoned,
                });
            }
        }
    }
    cited
}

/// The specs of `specs/`, by file name; their problems go to `out`, their warnings to `warn`.
pub(super) fn read_specs(root: &Path, out: &mut Vec<String>, warn: &mut Vec<String>) -> Vec<Spec> {
    let mut names: Vec<String> = std::fs::read_dir(root.join("specs"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md") && n != "README.md" && n != "TEMPLATE.md")
        .collect();
    names.sort();
    let mut specs = Vec::new();
    for name in names {
        let file = format!("specs/{name}");
        let text = std::fs::read_to_string(root.join(&file)).unwrap_or_default();
        specs.push(parse(root, &file, &text, out, warn));
    }
    specs
}

fn problems(root: &Path, strict: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut warnings = Vec::new();
    let specs = read_specs(root, &mut out, &mut warnings);
    warnings.iter().for_each(|w| eprintln!("{w}"));
    let mut ids: BTreeMap<&str, &str> = BTreeMap::new();
    for spec in specs.iter().filter(|s| !s.id.is_empty()) {
        if let Some(other) = ids.insert(&spec.id, &spec.file) {
            out.push(format!(
                "{}: spec id `{}` is already used by {other}",
                spec.file, spec.id
            ));
        }
    }
    let files = test_files(root, &mut out);
    let cited = citations(root, &files, &mut out);
    let known: BTreeSet<&str> = specs
        .iter()
        .flat_map(|s| s.requirements.iter().map(|r| r.id.as_str()))
        .collect();
    for (id, cites) in cited.iter().filter(|(id, _)| !known.contains(id.as_str())) {
        // a managed test (under `.claude/`) may cite a spec that stays in the harness repository
        let foreign = |file: &str| {
            !strict && is_managed(file) && !ids.contains_key(id.split('-').next().unwrap_or(""))
        };
        for file in cites.iter().map(|c| &c.file).filter(|f| !foreign(f)) {
            out.push(format!("{file}: cites `{id}`, which no spec defines"));
        }
    }
    for spec in specs.iter().filter(|s| s.status == "accepted") {
        for req in spec.requirements.iter().filter(|r| !r.exempt) {
            if !cited.contains_key(&req.id) {
                out.push(format!(
                    "{}: requirement {}: no test cites `// spec: {}`",
                    spec.file, req.id, req.id
                ));
            }
        }
    }
    boundary_warnings(&specs, &cited);
    out
}

/// Whether a test file is a managed one, under `.claude/`.
fn is_managed(file: &str) -> bool {
    file.replace('\\', "/").starts_with(".claude/")
}

/// Whether a test file calls the code from inside: it sits under a `src` directory, outside the managed tests
/// (TRC-1, TRC-5).
fn is_internal(file: &str) -> bool {
    !is_managed(file) && Path::new(file).components().any(|c| c.as_os_str() == "src")
}

/// Warn, per spec, of the cited requirements whose every citation is internal, one at least giving no reason
/// (TRC-1 to TRC-4, TRC-6).
fn boundary_warnings(specs: &[Spec], cited: &BTreeMap<String, Vec<Cite>>) {
    for spec in specs {
        let internal: Vec<&str> = spec
            .requirements
            .iter()
            .filter(|r| !r.exempt)
            .filter(|r| {
                cited.get(&r.id).is_some_and(|cites| {
                    cites.iter().all(|c| is_internal(&c.file)) && cites.iter().any(|c| !c.reasoned)
                })
            })
            .map(|r| r.id.as_str())
            .collect();
        if !internal.is_empty() {
            let plural = if internal.len() == 1 { "" } else { "s" };
            eprintln!(
                "warning: {}: {} requirement{plural} proven only by tests under `src` with no reason: {} \
                 (give it as `// spec: ID (internal: <reason>)`)",
                spec.file,
                internal.len(),
                internal.join(", ")
            );
        }
    }
}

pub(super) fn main(args: &[String]) -> u8 {
    let args: Vec<String> = args.to_vec();
    if let Some(flag) = args.iter().find(|a| a.starts_with('-') && *a != "--strict") {
        eprintln!("unknown option `{flag}` (usage: cardinal run check-specs [--strict] [root])");
        std::process::exit(2);
    }
    if args.iter().filter(|a| *a != "--strict").count() > 1 {
        eprintln!("usage: cardinal run check-specs [--strict] [root]");
        std::process::exit(2);
    }
    let strict = args.iter().any(|a| a == "--strict");
    let root = PathBuf::from(
        args.iter()
            .find(|a| *a != "--strict")
            .map_or(".", String::as_str),
    );
    let found = problems(&root, strict);
    for p in &found {
        println!("{p}");
    }
    std::process::exit(i32::from(!found.is_empty()));
}
