---
id: REP
status: draft
adrs: [0002, 0006]
---

# Report of the harness evolution by revision

## Intent

`.claude/transcripts/harness_report.rs` joins the outputs of the transcript readers (`session_metrics.rs`,
`rule_breaches.rs --summary`, `delivered_quality.rs --summary`, `fixed_context.rs`) into one markdown table per axis
(efficiency, quality, rules, skills, fixed context), one row per group of consecutive harness revisions, with the
confounders and the sample size next to the numbers, so `skill:evaluate-harness` shows the trend without computing
it. It reports trends, never a pass or a fail.

## Requirements

- **REP-1** (deterministic) `fixed_context.rs [projects_dir]` must print a header line then one tab-separated row per project (EVA-7) of the sessions of `projects_dir` (default `~/.claude/projects`) and harness revision (EVA-8) used by one of them, with the columns `project`, `revision`, `commit`, `claude_md_bytes`, `agents_md_bytes`, `skill_description_bytes`, `skills`, sorted by project then by the earliest start of the revision's sessions; the revision `` (empty) has no row.
- **REP-2** (deterministic) The `commit` of a row must be the abbreviated hash of the oldest commit of the first-parent history of the project's default branch at which the revision is in force (EVA-8, for a `self:` revision its own commit), and the byte columns must be read from the tree of that commit: the sizes of `CLAUDE.md` and `AGENTS.md` (0 when absent), and the sum over the `.claude/skills/*/SKILL.md` files of the bytes of their top-level front matter `name` and `description` values (quotes removed, a value over several lines counted with one byte per line break); `skills` lists the skill directory names joined by `,`, sorted. A revision with no such commit (pinned on another branch, history unreadable) has its columns after `revision` empty.
- **REP-3** (deterministic) `harness_report.rs --sessions <file> [--breaches <file>] [--quality <file>]... [--context <file>] [--min-sessions <n>] [--anonymous]` must read the rows of `session_metrics.rs` (EVA-1), the summary of `rule_breaches.rs` (BRE-3), one or more summaries of `delivered_quality.rs` (QUA-9) and the rows of `fixed_context.rs` (REP-1), and print one markdown table per axis, each under a `## <Axis>` heading, whose first two columns are `group` and `sessions`. A file whose header is not the expected one, a missing `--sessions`, an unknown option, or a `--min-sessions` that is not a whole number from 1 to 1000 (default 10) must be refused with an error and a non-zero exit.
- **REP-4** (deterministic) The revisions must be ordered by the earliest `start` of their sessions (the order they were used in; the empty revision, and a revision with no session, are left out of every table) and cut into groups of consecutive revisions, each closed as soon as it holds at least `--min-sessions` sessions; a group is named `<first revision>` or `<first revision>..<last revision>`, a revision of 40 hexadecimal digits (after `self:` or not) written with its first 7, and the last group, when it holds fewer sessions, shows its `sessions` and `too few` in each other cell of every table.
- **REP-5** (deterministic) The efficiency table (`## Efficiency`) must give per group `sessions`, `issues` (distinct pairs of project and issue of EVA-6), `projects`, `models` and `versions` (each `name=n`, `n` the sessions of the group that used it, most frequent first, then by name; `projects` is only the count of distinct projects with `--anonymous`), then the median per session (the mean of the two middle values for an even count, rounded half up to the unit) of `turns`, `tool_calls`, `output_tokens`, the sum of `input_tokens`, `cache_read_tokens` and `cache_creation_tokens` (column `input_tokens`), `validations` and `other_messages`.
- **REP-6** (deterministic) The quality table must give per group the sums of the `delivered_quality.rs` summary columns over its revisions and files (an empty value making its sum empty, as QUA-9), with the ratios `fixes_after` per issue and `review_findings` per review; the rules table one row per rule and group with `applied`, `breached`, `breaches`, `blocked` summed over its revisions, and the share `breached` over `applied`.
- **REP-7** (deterministic) The skills table must give per group the calls of each skill (EVA-5 `skills` column, summed), the skill names listed by `fixed_context.rs` for its revisions with no call in the group (`unused`), and the hook refusals by hook; the fixed context table per group the mean over its `fixed_context.rs` rows of each byte column and of their total, rounded to the unit.
- **REP-8** (deterministic) The output must hold no text of a message, of a commit or of an issue, and with `--anonymous` no project name nor issue stem either (only revisions, model and version names, skill and hook names, counts and ratios); `fixed_context.rs` and `harness_report.rs` must carry the `// transcripts.rs sha256: <hex>` line of EVA-13 when they include `transcripts.rs`.

## Scenarios

- (REP-4) Given revisions `a` (6 sessions), `b` (5) and `c` (3) used in that order with the default threshold, when the report is printed, then the groups are `a..b` (11 sessions) and `c`, marked `too few` with 3 sessions.
- (REP-4) Given a consumer pinned to an old revision used after newer `self:` revisions, when the report is printed, then the old revision takes its place by first use and the project mix of its group shows it.
- (REP-2) Given a project whose `harness.toml` moved from `r1` to `r2` in a commit that also grows `AGENTS.md`, when the row of `r2` is printed, then `agents_md_bytes` is the size in that commit.
- (REP-6) Given a rule that applied in 8 sessions of `a` and 4 of `b`, broken in 1 of each, when `a..b` is a group, then its row reads `applied` 12, `breached` 2.

## Non-goals

- A verdict or a threshold on the agent's behaviour: numbers become targets once shown; the report gives trends.
- Causes: model, Claude Code version and project change with the revision; the report shows them, a replay
  (`issues/replay_done_issues_to_compare_harness_revisions.md`) isolates them.
- The false positives of the hook refusals and the skills called where another fits: they need judgment.
- The text a hook prints in a session: the shared hooks print only on a refusal, already counted.
- Archiving the report: the session rows have their archive (EVA-11) and the git signals can be recomputed.
