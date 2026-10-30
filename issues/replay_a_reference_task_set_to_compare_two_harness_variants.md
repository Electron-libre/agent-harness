---
rank: 175
why: 'last of the new ones: the costliest; the observational verdicts come first and show whether it is needed'
---
# Replay a reference task set to compare two harness variants

# Goal
Every verdict of Cardinal is observational: it compares revisions over the real sessions that happened to run on
them, with different issues, projects, models and versions. A change of a skill or of the model can only be judged
after the fact, and a regression that shows once in two runs is easily missed. The outcome: a small set of fixed tasks
on a test repository, run headless several times against two variants of the harness (or two setups), reported with
the same metrics as `cardinal-report`.

# Findings
* `cardinal-report` groups sessions by revision and names the confounders (models, versions, project mix): REP-4,
  REP-5; it has no controlled comparison.
* The article on folding subagents recommends about ten reference tasks to start, graded on the final answer, each run
  several times: in its support-agent test a regression appeared in only one run of two.
* Not verified: the cost of a run of a full issue in headless mode (`claude -p`) on the user's plan.

# Approach
1. Measure first: one task (a small issue of a fixture repository) run headless twice, its duration, tokens and the
   effort to grade it.
2. Candidate: a fixture repository with about ten small issues, a recipe that syncs a given Cardinal revision into a
   copy, runs each issue N times and feeds the transcripts to `session-metrics` and `delivered-quality`.
3. Grading: the tests of the fixture pass, the review gate held, the rules of `rule-breaches` kept.
Likely over the size limit: split at `issue-workflow` step 2 (fixture, runner, report).

# Milestone
Lean agent loop

# Resources
* `specs/evaluation.md`, `specs/delivered-quality.md`, `specs/cardinal-report.md`.
* https://claude.com/fr/resources/articles/building-agents-when-to-fold-subagents-into-your-main-agent-loop

# Hints
* No network in the unit tests: the runner is a recipe, not a test of `cargo test`.
* The tasks must need no answer from the user, or the run stops at the stop of step 2: hand the plan pre-validated.
