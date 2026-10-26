# Extract per session metrics from the transcripts

# Goal
Know, for each agent session, how much it cost and how much the user had to step in, tagged with what may explain
a difference (harness revision, model, Claude Code version, project, issue). This is the base the other
evaluation issues read; it measures, it does not judge.

# Findings
* 2026-10-26, transcripts of every project: each line carries `sessionId`, `timestamp`, `cwd`, `gitBranch` and
  `version` (Claude Code, 10 distinct values from 2.1.257 to 2.1.300); assistant lines carry `message.model`
  (3 models seen) and `message.usage` (input, output, thinking, cache read and cache creation tokens).
* Lines also have undocumented types (`atis-latch`, `api-request-blob`, `api-request-shape`, `queue-operation`):
  the format moves between versions.
* Hook refusals are visible as tool results with `is_error` and a `PreToolUse:<tool> hook error: [...]` text
  (seen: the review gate refusing a commit).
* Skill calls are `tool_use` blocks named `Skill` with `input.skill`.
* The transcripts do not record the harness revision; the consumer's `.claude/harness.toml` pins it (`rev`,
  ADR 0002), so `cwd` plus `timestamp` joined with the git history of that file gives the revision in force.
* `files/.claude/skills/audit-instructions/audit_sources.rs` already walks the projects and reads the typed messages.

# Approach
1. A script (or a `harness` subcommand: to decide) that prints one row per session: project, issue (from the
   worktree or branch), harness revision, model, Claude Code version, duration, turns, tool calls, tokens by kind,
   hook refusals by hook, skill calls, typed messages split into plain validations and the rest.
2. Share the transcript reading with `audit_sources.rs` rather than duplicating it.
3. Ignore unknown line types and missing fields, and count what was skipped, so a format change shows instead of
   silently skewing the numbers.
4. Output text only (TSV or JSON), no message text, so the rows can be archived or committed as aggregates.
5. Archive the rows of each session outside the repositories, so the history no longer depends on the retention of
   the transcripts (moved here from `keep_the_session_transcripts_for_the_evaluation`, which warns of a short
   retention, SYN-28).

# Specs
* A new `specs/evaluation.md` (id to pick) for the extracted fields, the revision join and the tolerance to unknown
  lines, written at `issue-workflow` step 1.

# Resources
* `files/.claude/skills/audit-instructions/audit_sources.rs`, `files/.claude/skills/audit-instructions/SKILL.md`
* `docs/adr/0002-rev-pins-the-resolved-commit.md`, `docs/adr/0003-one-worktree-per-issue.md`
* `issues/done/keep_the_session_transcripts_for_the_evaluation.md`

# Hints
* Sub-agent sessions (`isSidechain`, per session sub-directories) belong to their parent session; count them
  with it.
* A session can span several issues or none; do not force a mapping.
* Tests use fixture transcripts in `tempfile::tempdir()`, never the real ones.

# Tracking

Plan: validated

## Understanding
One TSV row of numbers per session (EVA-1), sub-agents folded in, unknown lines counted, harness revision joined
from the git history of `.claude/harness.toml`, rows merged into an archive outside the repositories. Out of scope:
judging sessions, prices. Means: a managed `rust-script` sharing a module with `audit_sources.rs` (ADR 0006 draft),
not a `harness` subcommand.

## Plan
1. [x] Shared reader and per session rows: `files/.claude/transcripts/transcripts.rs` (moved out of
   `audit_sources.rs`) and `session_metrics.rs` with every column but `revision`; verified by
   `test_session_metrics.rs` and `test_audit_sources.rs` on fixture transcripts; makes true EVA-1 to EVA-7, EVA-9,
   EVA-10, EVA-12.
2. [ ] Harness revision: the join of `start` with the history of `.claude/harness.toml` on the default branch;
   verified by fixture git repositories (pinned, self, unpinned, unreadable); makes true EVA-8.
3. [ ] Archive: `--archive <file>` merges the rows; verified by a fixture archive; makes true EVA-11 (spec
   `accepted`).

## Decisions
- Mode: autonomous
- Base commit: e144aa2baef8ec76887fbe0a5491d161a07ae1c9
- Means: a managed script in `files/.claude/transcripts/`, not a `harness` subcommand (ADR 0006).
- Revision of the harness repository itself: `self:<last commit touching files/>`.
- Archive: no default path; rows are archived only with `--archive <file>`.
- Plain validations: the list of EVA-4 plus the answers to a stop.

## Deliverable 1
- done: `transcripts.rs` (shared reading), `session_metrics.rs` (every column, `revision` left empty until
  deliverable 2), `audit_sources.rs` on the shared module, `test-harness` runs `.claude/transcripts/test_*.rs`,
  ADR 0006 accepted, architecture and README.
- red: the script did not exist, every row check failed; then the ai-title fixture line counted as a missing field.
- Spec changes found on the real transcripts: `thinking_tokens` column (the usage carries
  `output_tokens_details.thinking_tokens`); EVA-9 counts a missing `timestamp` on conversation lines only, since
  the metadata lines (`ai-title`, `atis-latch`, `last-prompt`, `mode`, `cost-state`, `api-request-blob`) never carry
  one (22800 false "missing" otherwise).
- Smoke run on the real transcripts: 194 sessions in under a second, 0 unreadable, 0 unknown, 0 missing.
