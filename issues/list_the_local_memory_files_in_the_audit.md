# List the local memory files in the audit

# Goal
`memory.rs` never reads `CLAUDE.local.md`, which Claude Code documents as project memory, personal and kept out of
git. `audit-sources` (AUD-1) therefore does not list the rules a user wrote there, and the audit can propose them
again. The audit should list it in each directory it reads, as it lists `CLAUDE.md`.

# Findings
* 2026-10-28, `high` review of `issues/done/resolve_the_instruction_files_as_a_session_loads_them.md`: reported on
  `loaded` in `memory.rs`.
* `issues/done/count_every_always_loaded_memory_file.md` left `CLAUDE.local.md` out of REP-2 and SYN-38 on purpose:
  it is not in the git tree `fixed-context` reads, and the measured consumers had none.
* Not measured: whether Claude Code 2.1.312 still loads `CLAUDE.local.md` at launch, and its imports.

# Approach
1. Measure first: a session in a scratch repository with a `CLAUDE.local.md` holding a token, in the working
   directory and in an ancestor.
2. If it loads, list it in `instruction_files` (AUD-1) only, keeping it out of `measure` (REP-2, SYN-38), or count it
   in the work-tree budget too; the choice is the user's.

# Specs
* `specs/instruction-audit.md` AUD-1.

# Resources
* `crates/harness/src/memory.rs`, `crates/harness/src/tools/audit_sources.rs`
* `issues/done/count_every_always_loaded_memory_file.md`

# Tracking

Plan: validated

## Understanding
List `CLAUDE.local.md` and its imports among the instruction files of each directory the audit reads (AUD-1). Out of
scope: the fixed-context measure (REP-2, SYN-38), unless the user chooses otherwise (question 1).

## Measure (2026-10-28, Claude Code 2.1.312, `claude -p` in scratch repositories)
* A session in `repo/a/b` loads the `CLAUDE.local.md` of `repo/a` and of `repo` at launch, as it loads `CLAUDE.md`.
* A `CLAUDE.local.md` of the working directory importing `@inc.md` loads `inc.md`; from an ancestor, the import
  waits for the approval a `-p` session never gives, as the imports of `CLAUDE.md` do.

## Plan
1. [x] Local memory in the audit: `memory.rs` adds `CLAUDE.local.md` to the roots of `instruction_files` for a
   project directory, not to `measure`; amended AUD-1; verified by the `audit_sources` test of every instruction
   file (a `CLAUDE.local.md` importing a file); makes true AUD-1. ~50 lines.

## Decisions
- Mode: autonomous
- Base commit: db029397fbf432c2aad9b0c23c2723732b0f0e29
- `CLAUDE.local.md` listed by the audit only (AUD-1), never counted in REP-2 nor SYN-38 (user, 2026-10-28).

## Deliverable 1
- done: `audit-sources` lists `CLAUDE.local.md` and its imports in each directory it reads (`memory.rs`, a `local`
  file of the layout read by `instruction_files` only); AUD-1, the module doc and `audit-instructions` say so.
- | check | result |
  |---|---|
  | tests | pass |
  | lint | pass |
  | specs | pass |
  | docs | AUD-1, `audit_sources.rs` doc, `audit-instructions` skill |
  | code review | see below |
- red: AUD-1: `audit_sources_lists_every_instruction_file_once` lacked `CLAUDE.local.md` and `docs/local.md`.
- Code review: no findings
- not verified: whether a `CLAUDE.local.md` keeps Claude Code from reading `AGENTS.md` in place of `CLAUDE.md`
  (irrelevant to the audit, which lists `AGENTS.md` anyway).
- decisions needed: none.
