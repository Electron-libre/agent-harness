//! The files of a working tree as git lists them, shared by the scripts that scan a repository (`#[path]` include,
//! ADR 0006): tracked and untracked, without the ignored ones. A worktree nested in the scanned one (ADR 0008) is
//! never listed, whether git ignores its directory or sees an embedded repository there, so no scanner has to know
//! where the worktrees are.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The existing files under `dir`, from `git ls-files -co --exclude-standard`; `None` when `dir` is not in a git
/// work tree. An embedded repository that git does not ignore (a nested worktree or clone) is listed as its `dir/`
/// alone, and a submodule as its directory: both are dropped, with their content, as are the deleted tracked files.
pub fn repo_files(dir: &Path) -> Option<Vec<PathBuf>> {
    let out = Command::new("git").args(["ls-files", "-co", "--exclude-standard", "-z"]).current_dir(dir).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let files = out
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty() && !p.ends_with(b"/"))
        .map(|p| dir.join(String::from_utf8_lossy(p).as_ref()))
        .filter(|p| p.is_file())
        .collect();
    Some(files)
}
