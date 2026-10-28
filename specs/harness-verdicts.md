---
id: VRD
status: draft
adrs: [0002, 0010]
---

# Verdicts on the recorded changes of the harness

## Intent

A commit of the harness source that changes the shared files records the effect it is expected to have, as a
metric of `harness-report` and a direction; `harness run harness-changes` lists these expectations with the
revisions in use that hold each change, and `harness-report --changes` gives each one a verdict from the groups
before and after it, `confounded` whenever something else changed too, so `skill:evaluate-harness` ties a trend to
the change that should have caused it, or says it cannot.

## Requirements

- **VRD-1** (deterministic) An expectation must be a line of a commit message of the form `Expected-effect: <metric> <direction>`, `<direction>` being `up` or `down` and `<metric>` one of `turns`, `tool_calls`, `output_tokens`, `input_tokens`, `validations`, `other_messages` (REP-5), `fixes_after_per_issue`, `findings_per_review` (REP-6), `total_bytes` (REP-7), or `breached_share:<rule>`, `<rule>` a name of lowercase letters, digits and `-`, as the rules of the `rule-breaches` summary (BRE-3), and the line holding nothing else.
- **VRD-2** (deterministic) `commit-check` must refuse, with a message naming the line, a commit message holding a line that starts with `Expected-effect:` and is not an expectation of VRD-1.
- **VRD-3** (deterministic) `harness-changes [repo] --sessions <file>` must print a header line then one tab-separated row per expectation of the non-merge commits of the first-parent history of the default branch of `repo` (default the current directory; the default branch as EVA-8 picks it) that change a path under `files/`, with the columns `change` (the full hash of the commit), `metric`, `direction` and `revisions`, sorted by the commit date of the change then by the order of the lines in its message.
- **VRD-4** (deterministic) `revisions` must list the distinct `revision` values of the sessions file, in the order of the earliest `start` of their sessions and joined by `,`, whose commit (the value itself, or what follows `self:`) exists in `repo` and has the change as an ancestor or is the change; a revision git cannot resolve in `repo`, `unpinned` and the empty revision are never listed.
- **VRD-5** (deterministic) With `--changes <file>` (the rows of VRD-3, any other header refused as REP-3), `harness-report` must print a `## Verdicts` table, after the other tables, with one row per change and metric, in the order of the file, and the columns `change` (its first 7 hexadecimal digits), `metric`, `expected` (the direction), `before`, `after` (group names of REP-4), `before_value`, `after_value` and `verdict`; the after group is the first group (REP-4) holding a revision of the row's `revisions`, the before group the group just before it, and the values the cells of the metric in the efficiency, quality, fixed context or rules table (`breached_share` of the rule's row, as a number) of these groups, empty when the table is not printed, the group is missing or the cell is empty.
- **VRD-6** (deterministic) `verdict` must be `no data` when a group or a value of the row is missing, else `confounded` when a group is `too few`, when the after group holds a revision absent from the row's `revisions`, when the most frequent name of `models`, of `versions` or of `projects` (REP-5, before `--anonymous` hides the projects) differs between the two groups, or when another change of the file has the same after group; else `helped` when the after value moved from the before value in the expected direction, `hurt` when it moved the other way and `no observed effect` when they are equal.
- **VRD-7** (deterministic) The output of `harness-changes` and of the `## Verdicts` table must hold no text of a commit message other than the metric and the direction of its expectations.
- **VRD-8** (deterministic) `harness-changes` must refuse with an error and a non-zero exit a missing `--sessions`, a sessions file whose header is not the one of EVA-1, an unknown option, or a `repo` that is not a git repository.

## Scenarios

- (VRD-3) Given a commit that changes only `docs/` with an `Expected-effect: turns down` line, when the rows are printed, then it has none.
- (VRD-4) Given sessions of revisions `r1`, `self:r2` and `r3`, the change being `r2` and `r3` a descendant of it, when the rows are printed, then `revisions` is `self:r2,r3`.
- (VRD-5) Given a change held by the revisions of the second group only, when the report is printed, then `before` is the first group and `after` the second.
- (VRD-5) Given a change held by a revision of the first group, when the report is printed, then `before` is empty and `verdict` is `no data`.
- (VRD-6) Given groups `a` and `b` of 10 sessions each with the same models, versions and projects, the change held by every revision of `b` and none of `a`, `findings_per_review` 1.50 in `a` and 0.80 in `b`, when the expectation is `findings_per_review down`, then `verdict` is `helped`.
- (VRD-6) Given the same groups where the after group also holds a session of a project pinned to a revision without the change, then `verdict` is `confounded`.
- (VRD-2) Given a message with `Expected-effect: speed up`, when it is checked, then `commit-check` refuses it.

## Non-goals

- A threshold on the agent's behaviour: the expectation names a direction, never a value to reach.
- A significance test: a verdict compares two groups; the evaluation reads `confounded` and `too few` as no
  conclusion, and a trend over two groups or more (`skill:evaluate-harness`).
- Replaying a task under two revisions: the evaluation stays on real sessions of real projects.
- Expectations recorded outside the commit messages (an issue section, a file under `docs/`), or changed after the
  commit: the history is the record.
- The changes of a consumer's own files: only the harness source is read.
