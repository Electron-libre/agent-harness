# List the local memory files in the audit

# Goal
`memory.rs` never reads `CLAUDE.local.md`, which Claude Code documents as project memory, personal and kept out of
git. `audit-sources` (AUD-1) therefore does not list the rules a user wrote there, and the audit can propose them
again. The audit should list it in each directory it reads, as it lists `CLAUDE.md`.

# Findings
* 2026-10-28, `high` review of `issues/resolve_the_instruction_files_as_a_session_loads_them.md`: reported on
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
