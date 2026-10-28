# Restore the git source detection

# Goal
Commit `e5352ff` (clippy pedantic) rewrote `is_git_url` and changed which sources count as a git URL rather than a
local directory. `sync`, `update`, `cardinal run` and the launcher all branch on it, so a source can switch from a
clone to a local read (or the reverse) with no change of the project. Bring back the old decision, or decide and
specify a new one, and test it.

# Findings
* Before: `source.contains("://") || source.starts_with("git@") || source.ends_with(".git")`. After
  (`crates/cardinal/src/lib.rs:358`): the last term became `Path::new(source).extension()` equal to `git`, case
  insensitive.
* `Path::extension` of `/x/.git` is none (`.git` is a stem): that source was cloned before, is now read as a local
  directory. `../x.git/` (trailing slash) and `X.GIT` go the other way, from local directory to clone.
* Callers: `lib.rs:706`, `lib.rs:734`, `lib.rs:982` (`update` refuses a local source), `run.rs:87` (cached binary
  vs local build), `launcher.rs:68`.
* Found by the `high` review of `rename_the_harness_named_tools_to_cardinal`; no spec names the rule.

# Approach
1. Restore `ends_with(".git")` with an `#[allow]` and the reason, unless a source form above should change.
2. Write the rule as a requirement (`specs/sync.md`) and test it on the forms above.

# Specs
* `specs/sync.md`: a requirement for what a git source is, new.

# Resources
* `crates/cardinal/src/lib.rs` (`is_git_url`), `run.rs`, `launcher.rs`
* `git show e5352ff -- crates/cardinal/src/lib.rs`
