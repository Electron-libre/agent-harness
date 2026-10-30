---
rank: 66
why: 'after the cache share, before the exploration verdict: gives it the cost of a finished issue rather than of a session'
---
# Report the tokens spent per closed issue

# Goal
The efficiency table gives medians per session (REP-5). An issue often takes several sessions, and a session can stop
before anything lands, so a change that makes sessions shorter but more numerous looks cheaper while each delivered
issue costs more. The metric that matters is the cost of a task done: the tokens spent on an issue from its plan to
its close. The expected outcome: the report gives the tokens per closed issue next to the delivered quality.

# Findings
* `delivered-quality` already ties each closed issue of `issues/done/` to its sessions and to the events of those
  sessions that count for it (QUA-7, QUA-8), and gives per issue `sessions`, `failed_checks`, `validations`,
  `other_messages` (QUA-1); it reads no token.
* The quality table sums these per group and divides some by `issues` (REP-6).

# Approach
1. Add token columns to `delivered-quality` (input, cache read, output), summed over the messages of the issue's
   sessions that count for the issue (QUA-8), and a ratio per issue in the quality table.
2. Or join `session-metrics` rows to the issues by branch (EVA-6) in `cardinal-report`: simpler, but loses the
   sessions on the default branch and counts a whole session even when part of it worked on another issue.
Then the metric for `Expected-effect:` (VRD-1).

# Specs
* `specs/delivered-quality.md`: new columns of QUA-1 and their rule.
* `specs/cardinal-report.md`: REP-6 ratios; `specs/cardinal-verdicts.md`: VRD-1.

# Milestone
Lean agent loop

# Resources
* `specs/delivered-quality.md` (QUA-1, QUA-7, QUA-8), `specs/cardinal-report.md` (REP-6).
* `crates/cardinal/src/tools/delivered_quality.rs`, `transcripts.rs`.
* `issues/measure_the_main_context_peak_of_an_advised_message.md`: the usage of an advised message counts its
  iterations; the same trap applies to a token sum.

# Hints
* An abandoned issue never reaches `issues/done/`: its tokens are lost to this metric; say so in a non-goal.
