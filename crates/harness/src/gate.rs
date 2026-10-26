//! The native git `pre-commit` hook that runs the review gate of the project (`specs/sync.md`, SYN-18 to SYN-23).
//!
//! The hook is a small `sh` script that starts the managed `require_code_review.rs native` of the project, then the
//! `pre-commit` the user already had (moved to `pre-commit.user`, never overwritten).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{bail, Result};

use crate::{is_executable, set_executable, write, Outcome};

#[cfg(unix)]
use std::os::unix::fs::symlink;

/// The review hook of the project, relative to its root: the gate exists only where this managed file does.
pub const REVIEW_HOOK: &str = ".claude/hooks/require_code_review.rs";
/// The `permissions.deny` rules of `.claude/settings.json` that keep the review waiver from the agent (REV-19).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "read by `check` once SYN-24 is delivered")
)]
pub const DENY_RULES: [&str; 3] = [
    "Bash(just waive-review)",
    "Bash(just waive-review *)",
    "Edit(//**/claude-review-waived)",
];
/// Name of the gate in a report.
pub const GATE: &str = "native review gate";
/// The line that tells the installed hook from one of the user.
const MARK: &str = "# harness: native review gate";
/// Where a `pre-commit` of the user goes, and is run from.
const CHAINED: &str = "pre-commit.user";

/// The managed hook. Git runs it from the top of the work tree; a missing review hook or `rust-script` is a warning,
/// not a refusal, so a branch or a machine without the harness can still commit.
const SCRIPT: &str = r#"#!/bin/sh
# harness: native review gate (installed by `harness sync`, do not edit)
hook=.claude/hooks/require_code_review.rs
if [ -f "$hook" ] && command -v rust-script >/dev/null 2>&1; then
    rust-script "$hook" native || exit $?
else
    echo "harness: review gate skipped, $hook or rust-script is missing" >&2
fi
case "$0" in */*) dir="${0%/*}" ;; *) dir=. ;; esac
user="$dir/pre-commit.user"
if [ -x "$user" ]; then
    exec "$user" "$@"
fi
exit 0
"#;

/// What the hooks directory holds as `pre-commit`.
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Missing,
    /// A `pre-commit` that is not ours.
    Foreign,
    /// Ours, but edited or no longer executable.
    Modified,
    UpToDate,
    /// `core.hooksPath` sends git to another directory: the gate is not installed there.
    Redirected,
}

/// What `sync` will do about the gate, decided before anything is written.
#[derive(Debug)]
pub struct Plan {
    hooks: PathBuf,
    state: State,
}

/// The output of `git args` run in `dir`, `None` when it fails. The repository is the project's, not the one a hook or an
/// alias that runs `sync` is in.
fn git_in(dir: &Path, args: &[&str]) -> Option<String> {
    let out = git_raw(dir, args)?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn git_raw(dir: &Path, args: &[&str]) -> Option<Output> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .ok()
}

/// Whether `core.hooksPath` is set for the repository of `root` (`git config` exits 1 for an unset key).
fn hooks_path_set(root: &Path) -> bool {
    // any other failure (an unreadable config) is treated as set: better no gate than one in the wrong directory
    git_raw(root, &["config", "--get", "core.hooksPath"])
        .is_none_or(|out| out.status.code() != Some(1))
}

/// The hooks directory of the git repository of `root`, `None` when `root` is not the top of one.
fn hooks_dir(root: &Path) -> Option<PathBuf> {
    let git = |args: &[&str]| git_in(root, args);
    let top = fs::canonicalize(git(&["rev-parse", "--show-toplevel"])?).ok()?;
    if top != fs::canonicalize(root).ok()? {
        return None;
    }
    // for a linked worktree this is the hooks directory of the common git directory
    Some(root.join(git(&["rev-parse", "--git-path", "hooks"])?))
}

pub fn state(hooks: &Path) -> State {
    let path = hooks.join("pre-commit");
    match fs::read_to_string(&path) {
        Err(_) if fs::symlink_metadata(&path).is_err() => State::Missing,
        // the mark is the second line, so a hook of the user that only quotes it is not taken for ours
        Ok(text) if text.lines().nth(1).is_some_and(|l| l.starts_with(MARK)) => {
            if text == SCRIPT && is_executable(&path) {
                State::UpToDate
            } else {
                State::Modified
            }
        }
        _ => State::Foreign,
    }
}

/// The hooks directory of the project and what its `pre-commit` is, `None` when the project is not expected to have the
/// gate (`expected` is false: the review hook is not among its managed files) or is not the top of a git repository.
/// One decision for `sync` and `check`, so they never disagree about where the gate goes.
fn inspect(root: &Path, expected: bool) -> Option<(PathBuf, State)> {
    if !expected {
        return None;
    }
    let hooks = hooks_dir(root)?;
    let state = if hooks_path_set(root) {
        State::Redirected
    } else {
        state(&hooks)
    };
    Some((hooks, state))
}

/// The line `check` prints when the project should have the gate and does not (or not as managed), `None` when it is
/// there, cannot be installed (`core.hooksPath`, no repository) or is not expected. `expected` tells whether the review
/// hook is among the managed files, whether or not it was deleted since.
#[must_use]
pub fn problem(root: &Path, expected: bool) -> Option<String> {
    match inspect(root, expected)?.1 {
        State::Missing | State::Foreign => Some(format!("missing: {GATE}")),
        State::Modified => Some(format!("modified: {GATE}")),
        State::UpToDate | State::Redirected => None,
    }
}

/// Decide what to do for the project at `root`, whose managed files include the review hook when `has_review_hook`.
///
/// # Errors
/// When a `pre-commit` of the user must move to `pre-commit.user` and that file exists already.
pub fn plan(root: &Path, has_review_hook: bool) -> Result<Option<Plan>> {
    let Some((hooks, state)) = inspect(root, has_review_hook) else {
        return Ok(None);
    };
    if state == State::Foreign && hooks.join(CHAINED).exists() {
        bail!(
            "{} and {} both exist: merge them by hand, `harness sync` never overwrites a hook of yours",
            hooks.join("pre-commit").display(),
            hooks.join(CHAINED).display()
        );
    }
    Ok(Some(Plan { hooks, state }))
}

impl Plan {
    /// What the report says, in order.
    #[must_use]
    pub fn outcomes(&self) -> Vec<(String, Outcome)> {
        let mut out = Vec::new();
        if self.state == State::Redirected {
            let why = "not installed: core.hooksPath is set, so git reads its hooks elsewhere; add `rust-script .claude/hooks/require_code_review.rs native` to your own pre-commit, run from the top of the work tree";
            return vec![(GATE.to_string(), Outcome::Skipped(why))];
        }
        if self.state == State::Foreign {
            out.push(("pre-commit -> pre-commit.user".to_string(), Outcome::Moved));
        }
        let outcome = match self.state {
            State::Missing | State::Foreign => Outcome::Added,
            State::Modified => Outcome::Updated,
            State::UpToDate | State::Redirected => Outcome::Unchanged,
        };
        out.push((GATE.to_string(), outcome));
        out
    }

    /// Write the hook.
    ///
    /// # Errors
    /// When a file cannot be moved or written.
    pub fn install(&self) -> Result<()> {
        if matches!(self.state, State::UpToDate | State::Redirected) {
            return Ok(());
        }
        let hook = self.hooks.join("pre-commit");
        // the new hook is complete before the hook of the user moves, so a failure never leaves the repository
        // with no hook at all
        let staged = self.hooks.join("pre-commit.harness-new");
        write(&staged, SCRIPT.as_bytes())?;
        set_executable(&staged, true)?;
        if self.state == State::Foreign {
            let chained = self.hooks.join(CHAINED);
            if fs::symlink_metadata(&hook)?.is_symlink() {
                // a relative link would point elsewhere once moved: keep the target it reaches now
                let target = fs::canonicalize(&hook)?;
                fs::remove_file(&hook)?;
                symlink(&target, &chained)?;
            } else {
                fs::rename(&hook, &chained)?;
            }
        } else if self.state == State::Modified {
            // never write through a link
            fs::remove_file(&hook)?;
        }
        fs::rename(&staged, &hook)?;
        Ok(())
    }
}
