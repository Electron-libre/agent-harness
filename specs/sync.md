---
id: SYN
status: draft
adrs: [0002]
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

## Scenarios


- (SYN-3) Given a project whose file was never synced but differs from the source, when `sync` runs, then it is a conflict: the file is not ours yet.
- (SYN-6) Given a project whose file already equals the source but is not in the lock, when `sync` runs, then it is locked and reported as unchanged.
- (SYN-11) Given a project already at the resolved commit, when `update` runs, then `rev` is rewritten unchanged, the `sync` reports nothing and the run succeeds.
- (SYN-12) Given a `rev` line with a trailing comment from an earlier update, when `update` runs, then the comment is replaced by the new requested name.

## Non-goals

- Merging a local change with a source change: a drifted file is fixed in the central repository.
- Fetching more than a branch, tag or commit of a git source.
