//! The test files that cite no requirement of an `accepted` spec, largest first, to rank the capabilities worth a
//! spec read back from their code and tests (`skill:write-spec`).
//! Usage: `just spec-coverage` (from the repo root; an optional argument is another root). Prints `<lines>\t<path>`
//! per file and exits 0: a report, not a check (COV-8). The files are those `check-specs` reads its citations from.

use super::check_specs::{citation, read_specs, test_files};

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The `(line count, path)` of each test file citing no requirement of an `accepted` spec, largest first (COV-1 to
/// COV-5); `Err` when the files cannot be listed.
fn uncovered(root: &Path) -> Result<Vec<(usize, String)>, String> {
    // the problems and warnings of the specs are `check-specs`' business
    let accepted: BTreeSet<String> = read_specs(root, &mut Vec::new(), &mut Vec::new())
        .into_iter()
        .filter(|s| s.status == "accepted")
        .flat_map(|s| s.requirements.into_iter().map(|r| r.id))
        .collect();
    let mut errors = Vec::new();
    let files = test_files(root, &mut errors);
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let mut found: Vec<(usize, String)> = files
        .iter()
        .filter_map(|file| uncovered_file(root, file, &accepted))
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    Ok(found)
}

/// The line count and relative path of `file` when it holds a `#[test]` and cites no id of `accepted`.
fn uncovered_file(
    root: &Path,
    file: &Path,
    accepted: &BTreeSet<String>,
) -> Option<(usize, String)> {
    let text = std::fs::read_to_string(file).ok()?;
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let covered = lines
        .iter()
        .filter_map(|l| l.strip_prefix("// spec:"))
        .any(|list| citation(list).0.any(|id| accepted.contains(id)));
    if covered || !lines.contains(&"#[test]") {
        return None;
    }
    let name = file
        .strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string();
    Some((lines.len(), name))
}

pub(super) fn main(args: &[String]) -> u8 {
    if args.len() > 1 || args.iter().any(|a| a.starts_with('-')) {
        eprintln!("usage: harness run spec-coverage [root]");
        return 2;
    }
    let root = PathBuf::from(args.first().map_or(".", String::as_str));
    match uncovered(&root) {
        Ok(found) if found.is_empty() => println!("no test file without an accepted spec"),
        Ok(found) => found
            .iter()
            .for_each(|(lines, file)| println!("{lines}\t{file}")),
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    }
    0
}
