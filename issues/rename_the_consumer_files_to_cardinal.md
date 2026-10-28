# Rename the consumer files to cardinal

# Goal
Rename the files the tool keeps in each consumer: `.claude/harness.toml` → `.claude/cardinal.toml` (and its
`[harness]` table), `.claude/harness.lock` → `.claude/cardinal.lock`, `harness.just` → `cardinal.just`, with `sync`
migrating a consumer that still has the old names.

# Findings
* `harness.toml` appears in 167 lines of 71 files, `harness.lock` in 14 lines, `harness.just` in 12, the `[harness]`
  table in 36 lines.
* The paths are constants in `crates/cardinal/src/lib.rs` (`CONFIG`, `LOCK`); `run.rs` finds the project by the
  nearest `.claude/harness.toml`; `install_cardinal.sh` and `gate.rs` read it too.
* Each consumer's `justfile` holds `import 'harness.just'`; that file is project-owned, not managed.

# Approach
1. `sync` (and `run`, `check`, `update`) read the old names when the new ones are missing, and `sync` renames them,
   rewriting the `import` line of the `justfile`.
2. Rename the paths in the code, `files/`, `seed/`, the specs, the skills and the docs.
3. Drop the fallback once every consumer has migrated (separate issue, or a dated note).

# Specs
* `specs/sync.md` (a new SYN-n for the migration of the old names) and the specs naming these files; amended at
  `issue-workflow` step 1.

# Resources
* `crates/cardinal/src/lib.rs`, `run.rs`, `gate.rs`, `budget.rs`
* `seed/.claude/harness.toml`, `files/harness.just`, `.github/install_cardinal.sh`
* ADR 0001, ADR 0002 (rev pins the resolved commit)

# Hints
* After `rename_the_binary_to_cardinal`.
* A consumer pinned to an old revision still runs the old binary, which knows only `harness.toml`: the migration
  must not leave a project that the old binary can no longer read before its `update`.
