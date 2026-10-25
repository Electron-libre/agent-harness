---
id: SYN
status: implemented
adrs: [0002, 0004]
---

# Harness synchronisation

## Intent

A project keeps the shared harness (skills, hooks, scripts) in step with the central repository without merging
by hand, and learns at once when a managed file drifted from it.

## Requirements

- **SYN-1** (deterministic) `sync` must copy every file under the source's `files/` to the same relative path of the project, executable bit included, and `check` must report a changed executable bit as a modification.
- **SYN-2** (deterministic) `sync` must overwrite a managed file that was not modified locally since the last sync when the source changed.
- **SYN-3** (deterministic) `sync` must not overwrite a managed file modified locally, must report it as a conflict and must fail, unless `--force` is given.
- **SYN-4** (deterministic) `sync` must copy a file under the source's `seed/` only when the project does not have it yet, and must never touch it afterwards.
- **SYN-5** (deterministic) `sync` must delete a managed file the source dropped when it was not modified locally, and must keep it, no longer managed, when it was.
- **SYN-6** (deterministic) `sync` must be idempotent: a second run reports nothing and leaves the project byte-identical.
- **SYN-7** (deterministic) `sync --dry-run` must report what it would do and must not write anything, lock included.
- **SYN-8** (deterministic) `check` must list every managed file modified or deleted since the last sync, and nothing else.
- **SYN-9** (deterministic) `init` must create the project's `harness.toml` with the source and the project-owned defaults of the source, and must refuse to overwrite an existing one.
- **SYN-10** (deterministic) `sync` and `check` must fail without deleting anything when the source has no `files/` directory, or when the lock cannot be read or names a path outside the project.

- **SYN-11** (deterministic) `update` must resolve the requested revision (a branch, a tag or a commit; the head of the source's default branch when none is given) to a full commit hash, write that hash to `rev` in the `[harness]` table of `harness.toml` with the requested name in a trailing comment, then run the `sync` against it.
- **SYN-12** (deterministic) `update` must change only the `rev` line of `harness.toml`: every other line and comment stays byte-identical, and a missing `rev` is inserted right after `source`. When the file cannot be edited that way (a quoted or dotted key), `update` must fail changing nothing.
- **SYN-13** (deterministic) `update` must leave the project untouched, `harness.toml` and lock included, when the `sync` would report a conflict (unless `--force` is given), when the revision is unknown or when the source cannot be read.
- **SYN-14** (deterministic) `update` must fail, changing nothing, when the source is a local directory, since it has no revision.
- **SYN-15** (deterministic) `update --dry-run` must report the target revision and what the `sync` would do, and must write nothing.
- **SYN-16** (deterministic) `update` must report the previous revision (or that there was none), the new one and the files changed.
- **SYN-17** (deterministic) When `rev` is a full commit hash that no branch or tag of the source points to any more, `sync` and `update` must try to fetch that commit by its hash, and use it when the source serves it. When the source does not, they must fail changing nothing, with an error that names the pinned commit as gone from the source and says to run `harness update`.
- **SYN-18** (deterministic) `sync` (so `init` followed by `sync`; `init` alone only writes `harness.toml`, the hook is not there yet) must install the native review gate in the hooks directory of the project's git repository: an executable `pre-commit` script that starts with a mark line and runs `rust-script .claude/hooks/require_code_review.rs native` from the work tree, failing the commit with that exit code; when the review hook or `rust-script` is missing, the script must warn on stderr and let the commit through.
- **SYN-19** (deterministic) When a `pre-commit` without the mark already exists, the installation must move it to `pre-commit.user` and the installed hook must run it after the review gate, passing its arguments and failing the commit when it fails; it must never overwrite the user's hook, and must fail changing nothing when a `pre-commit.user` already exists.
- **SYN-20** (deterministic) `sync` must reinstall the native gate when it is missing, no longer executable or different from the managed one, and report nothing when it is up to date (idempotent, as SYN-6); `--dry-run` must report the installation and write nothing.
- **SYN-21** (deterministic) For a linked worktree, or a project whose `.git` is a file, the gate must be installed in the `hooks` directory of the common git directory.
- **SYN-22** (deterministic) When `core.hooksPath` is set for the project, `sync` must install nothing, report that the native gate is not installed and why, and succeed; a project that is not a git repository, or has no `.claude/hooks/require_code_review.rs`, gets no gate and no error.
- **SYN-23** (deterministic) `check` must report `missing: native review gate` when the project is a git repository that has the review hook among its managed files (even deleted since), whose hooks directory lacks the managed gate and where `core.hooksPath` is unset, and `modified: native review gate` when it differs; it must not report it otherwise, nor when the `CI` environment variable is set (a fresh clone of a CI run has no git hooks and nobody commits there).

## Scenarios


- (SYN-3) Given a project whose file was never synced but differs from the source, when `sync` runs, then it is a conflict: the file is not ours yet.
- (SYN-6) Given a project whose file already equals the source but is not in the lock, when `sync` runs, then it is locked and reported as unchanged.
- (SYN-11) Given a project already at the resolved commit, when `update` runs, then `rev` is rewritten unchanged, the `sync` reports nothing and the run succeeds.
- (SYN-12) Given a `rev` line with a trailing comment from an earlier update, when `update` runs, then the comment is replaced by the new requested name.
- (SYN-17) Given an abbreviated hash or a name that does not resolve, the fetch by hash is not tried and the error stays "unknown revision".
- (SYN-19) Given a user's `pre-commit` that runs a linter, when `sync` runs, then the linter runs after the gate, and a refusal of the gate stops the commit before it.
- (SYN-22) Given `core.hooksPath` set to a versioned directory, then `sync` leaves it untouched and says the gate is not installed there.

## Non-goals

- Installing the gate under a `core.hooksPath` the project sets itself.
- Merging a local change with a source change: a drifted file is fixed in the central repository.
- Fetching more than a branch, tag or commit of a git source.
- Making `check` depend on the source: it compares the project with its lock only, so the pin cannot break it.
- Guaranteeing that a host serves an unreferenced commit: it depends on the host.
