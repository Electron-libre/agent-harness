# Count every always-loaded memory file in the fixed context

# Goal
The fixed context measure counts only the root `CLAUDE.md` and `AGENTS.md` (plus the skill descriptions), both in
`fixed-context` (REP-2, on a revision) and in the budget warning of `check`, `sync` and `update` (SYN-38, on the work
tree). Claude Code loads other project memory in every session too, so a project keeping its instructions there is
under-counted and its budget never warns. Make the shared measure count what a session actually loads.

# Findings
* Raised by the `high` review of `issues/budget_the_fixed_context_of_a_project.md`: a project whose instructions sit
  in `.claude/CLAUDE.md` measures 0 for `CLAUDE.md`.
* Claude Code documents `.claude/CLAUDE.md` and `CLAUDE.local.md` as project memory, and `@path` imports in a memory
  file; not yet checked which of them this repository's consumers use.
* `AGENTS.md` is counted although Claude Code loads it only through an import from `CLAUDE.md`; the measure assumes it.

# Approach
1. Measure first: which memory files and imports the local consumers use, and their sizes.
2. Count `.claude/CLAUDE.md` with the root `CLAUDE.md` in one shared function used by `tools/fixed_context.rs` and
   `budget.rs`, so REP-2 and SYN-38 stay one measure.
3. Candidate: follow the `@path` imports of the memory files (once each), instead of counting `AGENTS.md` blindly.
4. `CLAUDE.local.md` is personal and kept out of git: count it in the work tree only, or not at all.

# Specs
* `specs/harness-report.md` REP-2 and `specs/sync.md` SYN-38, amended together.

# Resources
* `crates/harness/src/tools/fixed_context.rs`, `crates/harness/src/budget.rs`
* `specs/harness-report.md`, `specs/sync.md`

# Hints
* Keep the two measures one: the budget is only useful if it agrees with what `evaluate-harness` reports.
