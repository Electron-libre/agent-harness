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
* `issues/keep_the_session_transcripts_for_the_evaluation.md`

# Hints
* Sub-agent sessions (`isSidechain`, per session sub-directories) belong to their parent session; count them
  with it.
* A session can span several issues or none; do not force a mapping.
* Tests use fixture transcripts in `tempfile::tempdir()`, never the real ones.
