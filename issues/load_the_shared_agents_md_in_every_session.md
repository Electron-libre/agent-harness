# Load the shared AGENTS.md in every session

# Goal
`files/AGENTS.md` holds rules every session should follow (the commit message format, the output rule "Reports to the
user"), but Claude Code does not load it in the consumers: by default it reads `AGENTS.md` only when the project has
no `CLAUDE.md` or a `CLAUDE.md` imports it. Its rules reach the context only when a skill sends the agent to read the
file. Make the shared rules part of what every session loads, or decide where they belong instead.

# Findings
* 2026-10-28, measured for `issues/count_every_always_loaded_memory_file.md`: both local consumers (this repository
  included) have a root `CLAUDE.md` that names `AGENTS.md` in backticks only, no `@AGENTS.md` import; neither has a
  `.claude/CLAUDE.md`, `CLAUDE.local.md` nor `.claude/rules/`.
* Claude Code documentation (memory page, read with v2.1.312): with the default `instructionFiles` setting
  `claude-md-or-agents-md`, a `CLAUDE.md` in the working directory or above it makes Claude read the `CLAUDE.md` files
  only; `AGENTS.md` loads through an import, a symbolic link, or the user setting `claude-md-and-agents-md` (ignored in
  project settings).
* The session context of this repository shows `CLAUDE.md` and no `AGENTS.md`.
* `CLAUDE.md` is owned by each project (neither managed nor seeded), so `sync` cannot add the import itself.

# Approach
1. Measure first: in the transcripts, how often the agent followed the "Reports to the user" format and the commit
   format without a skill that sends it to `AGENTS.md` (`rule-breaches` may already count it).
2. Candidates (can be combined):
   1. `check` warns when a project's `CLAUDE.md` neither imports `AGENTS.md` nor is one with it.
   2. Move the shared always-on rules to a managed `.claude/rules/<name>.md` without `paths`, which Claude Code loads
      at launch with no edit of the project's `CLAUDE.md`; keep `AGENTS.md` for other agents.
   3. `init` seeds a `CLAUDE.md` holding the import for a new project.
3. Weigh the fixed context each candidate adds against the budget (`fixed_context_budget`).

# Specs
* `specs/sync.md`: a new requirement for the warning or the managed rule file, if chosen.

# Resources
* `files/AGENTS.md`, `entries/.claude/settings.json`, `docs/architecture.md` (managed and seeded content)
* `specs/sync.md`, `specs/harness-report.md` REP-2 (what the fixed context measure counts as loaded)
* Claude Code documentation, memory page, section "AGENTS.md"

# Hints
* The fixed context measure counts `AGENTS.md` only when a session loads it: whichever candidate is chosen, the budget
  will report it.
