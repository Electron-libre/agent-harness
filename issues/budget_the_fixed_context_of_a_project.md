# Budget the fixed context of a project

# Goal
`harness run fixed-context` measures the context every session pays before any work (`CLAUDE.md`, `AGENTS.md`,
skill descriptions) but nothing bounds it, so it can only grow. Let a project set a budget and have
`harness check`, `sync` and `update` warn when the project goes over it.

# Findings
* `specs/harness-report.md` REP-1: `fixed-context` prints `claude_md_bytes`, `agents_md_bytes`,
  `skill_description_bytes` per project and revision, read from the transcripts' revisions; REP-7 reports the
  means per group. No requirement compares them to a limit.
* `harness check` already prints `warning:` lines that do not fail it (`crates/harness/src/settings.rs`).
* `google-research/harness2` (`harness2/core/tree.py`, `validate`) rejects a harness whose always-on text exceeds a
  character cap, before any run, and its prompts require paying for added always-on text by cutting some.

# Approach
1. Measure first: the current fixed context of this repository and of a few consumers, to pick a default.
2. A `fixed_context_budget` value in `[project]` of `.claude/harness.toml` (bytes; absent means no check).
3. `harness check`, `sync` and `update` print a `warning:` line naming the total, the budget and the largest part,
   reusing the measure of `fixed-context` on the work tree rather than on a past revision.
4. Candidate alternative: a test of this repository only, bounding what `files/` ships (`AGENTS.md`, skill
   descriptions), with no consumer setting.

# Specs
* `specs/sync.md` (the warnings of `check`, `sync`, `update`) or `specs/harness-report.md`; new requirements ids
  decided at `issue-workflow` step 1.

# Resources
* `crates/harness/src/tools/fixed_context.rs`, `crates/harness/src/settings.rs`, `specs/harness-report.md`
* `https://github.com/google-research/harness2`, `harness2/core/tree.py`

# Hints
* A warning, not a failure: the budget informs the user, it does not block a sync.
* The measure must not count what Claude Code loads on demand (skill bodies).
