# Warn when the default projects directory is missing

# Goal
With no projects directory given, the transcript tools read the `projects/` of the configuration directory
(EVA-15). When that directory does not exist, `session-metrics`, `rule-breaches`, `fixed-context` and
`audit-sources` print an empty result and exit 0 with no warning, so `evaluate-harness` reports zero sessions and
nothing says why. Tell the user the default directory is missing.

# Findings
* 2026-10-28, `high` review of `issues/done/read_the_transcripts_from_the_configuration_directory.md`: with
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

# Tracking

Plan: validated

## Understanding
When no projects directory is given and the default `projects/` of the configuration directory is not a
directory, the five transcript tools warn on stderr naming it, keep their valid empty output and exit 0 (approach 1).
Out of scope: a projects directory given explicitly (the user named it).

## Plan
1. [x] Warn in `projects_root` for the default only; verified by a CLI test running the five tools with
   `CLAUDE_CONFIG_DIR` set to an empty temp dir (warning on stderr, exit 0); makes true EVA-15 (amended).

## Decisions
- Mode: autonomous
- Answers: 1 a (warn, exit 0), 2 a (no warning for a directory given explicitly).
- Base commit: 9326407c2647637eab6826f68d9e6547c56d1f3e

## Deliverable 1
- done: `projects_root` warns on stderr naming the default `projects/` when it is not a directory; the five tools keep their empty output and exit 0.
- red: EVA-15: `session-metrics: ` (exit 0, empty stderr, no warning)
- Code review: no findings
- checks: `just check` ok (fmt, clippy, tests, specs); docs: `docs/usage.md`.
- not verified: none. decisions needed: none.
