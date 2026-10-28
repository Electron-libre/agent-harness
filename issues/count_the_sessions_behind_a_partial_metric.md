# Count the sessions behind a metric that some sessions lack

# Goal
The median `main_context_peak` of `harness-report` (REP-9) leaves out the sessions whose cell is empty (archived
before the column, EVA-17), but the `sessions` count of the group and its `too few` flag (REP-4), which the verdicts
read (VRD-6), count every session. A verdict on `main_context_peak` can then rest on one or two sessions and still
be `helped` or `hurt`. The expected outcome: a verdict on such a metric is `confounded` (or `no data`) when the
sessions that have it are fewer than `--min-sessions`.

# Findings
* Raised by the `high` review of `issues/delegate_the_exploration_of_the_issue_workflow_to_a_subagent.md` (commits
  `e311105`, `657bf10`).
* Only `main_context_peak` can be empty in a session row today; every other efficiency column is always filled.
* The archived rows lose their transcripts over time, so the groups before the column was added hold the most
  empty cells: the before group of the change that names `main_context_peak down` is the one at risk.

# Approach
1. Show next to the median the count of the sessions it was computed from (e.g. `main_context_peak` as `n=…` in
   a column of its own), so the reader sees the sample.
2. In the verdict, treat a group as `too few` for a metric when the sessions holding a value of that metric are
   fewer than `--min-sessions`.
Measure first how many archived rows lack the peak; if the archive is young, 1 may be enough.

# Specs
* `specs/harness-report.md` (REP-9), `specs/harness-verdicts.md` (VRD-6).

# Resources
* `crates/harness/src/tools/harness_report.rs` (`main_context_peak`, `verdicts`).
* `specs/harness-report.md`, `specs/harness-verdicts.md`, `specs/evaluation.md` (EVA-16, EVA-17).
