# Resolve the instruction files as a session loads them

# Goal
`memory.rs` resolves the memory files from the root of one tree: `audit-sources` (AUD-1) treats each session's
working directory as that root, and `fixed-context` (REP-2) the repository. Two cases differ from what a session
loads, so the audit can miss a rule the user wrote and propose it again:
* a session started in a subdirectory also loads the `CLAUDE.md` files of its ancestor directories, which the audit
  does not list;
* the imports of a memory file reached through a symbolic link are resolved against the directory of the link, not
  of the file it points to.

# Findings
* 2026-10-28, `high` review of `issues/done/list_the_always_on_rules_in_the_instruction_audit.md`: both reported on
  `audit_sources.rs` (one `WorkTree` per working directory) and `memory.rs` (`resolve(dir, …)` with `dir` the
  parent of the queued path).
* Not measured: which directory Claude Code resolves the import of a linked memory file against.

# Approach
1. Measure first: a session in a scratch repository with a linked `CLAUDE.md` importing a relative path, and one
   started in a subdirectory, to see what the session loads.
2. For the audit: walk the ancestors of the working directory up to the repository root (or the filesystem root)
   and list their memory files.
3. For the imports: resolve against the directory of the resolved file, in `memory.rs`, so that `fixed-context`
   follows; amend REP-2 if the measure says so.

# Specs
* `specs/instruction-audit.md` AUD-1, `specs/harness-report.md` REP-2.

# Resources
* `crates/harness/src/memory.rs`, `crates/harness/src/tools/audit_sources.rs`
