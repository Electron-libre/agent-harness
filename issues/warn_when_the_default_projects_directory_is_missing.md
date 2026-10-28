# Warn when the default projects directory is missing

# Goal
With no projects directory given, the transcript tools read the `projects/` of the configuration directory
(EVA-15). When that directory does not exist, `session-metrics`, `rule-breaches`, `fixed-context` and
`audit-sources` print an empty result and exit 0 with no warning, so `evaluate-harness` reports zero sessions and
nothing says why. Tell the user the default directory is missing.

# Findings
* 2026-10-28, `high` review of `issues/read_the_transcripts_from_the_configuration_directory.md`: with
  `CLAUDE_CONFIG_DIR` set to a directory with no `projects/` while the transcripts stay under the home directory's
  `.claude/projects`, `session-metrics` prints its header only and exits 0.

# Approach
1. Warn on stderr, naming the directory, when the default projects directory is not a directory; the output stays
   valid (an empty table).
2. Or fail with exit 2, as when the configuration directory cannot be located; `delivered-quality` keeps its empty
   transcript columns (QUA-7).

# Specs
* `specs/evaluation.md`: amends EVA-15.

# Resources
* `crates/harness/src/tools/transcripts.rs` (`projects_root`), `crates/harness/tests/transcripts_cli.rs`
