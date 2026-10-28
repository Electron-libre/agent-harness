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
