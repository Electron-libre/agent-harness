# Read the transcripts from the configuration directory

# Goal
The transcript tools (`audit-sources`, `rule-breaches`, `session-metrics`, `delivered-quality`, `fixed-context`)
default their projects directory to `$HOME/.claude/projects` and ignore `CLAUDE_CONFIG_DIR`, while `audit-sources`
reads the user-level instruction files from the configuration directory (`CLAUDE_CONFIG_DIR` first, AUD-2). With
`CLAUDE_CONFIG_DIR` set, one audit mixes the user files of one configuration with the sessions of another. Default
the projects directory to the `projects/` of the configuration directory.

# Findings
* 2026-10-28, `high` review of `issues/done/list_the_user_level_instruction_files_in_the_audit.md`: `projects_root`
  reads `HOME` only; `crate::config_dir()` now resolves the configuration directory for the retention warnings and
  AUD-2.

# Approach
1. `projects_root` defaults to `config_dir().join("projects")`, its error naming both variables.
2. Check the specs of the tools that state the default projects directory and amend them.

# Specs
* The specs of the transcript tools naming the default projects directory (`instruction-audit.md`,
  `evaluation.md`, `rule-breaches.md`, `delivered-quality.md`, `harness-report.md`).

# Resources
* `crates/harness/src/tools/transcripts.rs` (`projects_root`), `crates/harness/src/lib.rs` (`config_dir`)

# Tracking

Plan: validated

## Understanding
With no projects directory given, the five transcript tools read `<config dir>/projects`, the configuration directory
being the one of AUD-2 (`crate::config_dir()`); one shared requirement (EVA-15) states it, EVA-1, REP-1 and QUA-7 cite
it. Out of scope: `rule-breaches` and `audit-sources` specs, which name no default.

## Plan
1. [x] Default projects directory from the configuration directory: `projects_root` uses `config_dir()`, its error
   names `CLAUDE_CONFIG_DIR` and `HOME`; doc comments, `docs/architecture.md` and the `audit-instructions` skill
   updated; verified by a `session-metrics` binary test with `CLAUDE_CONFIG_DIR` and `HOME` pointing at different
   transcripts, and one with neither set; makes true EVA-15 (~80 lines).

## Decisions
- Mode: autonomous
- Base commit: dc0a7a24cdbb462d1505544deebb18fc05a06d83

## Deliverable 1
- done: `projects_root` defaults to `config_dir()/projects`; its error names both variables; usages, architecture
  and `audit-instructions` updated.
- red: EVA-15: `FAIL - the default projects directory is the one of CLAUDE_CONFIG_DIR`, `FAIL - with neither
  variable, the error names both`.
- Code review: no findings.
- not verified: none.
