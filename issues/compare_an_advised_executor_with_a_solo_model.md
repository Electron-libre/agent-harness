---
rank: 125
why: 'after the advisor count and the workflow guidance, and once enough sessions of each setup exist'
---
# Compare an advised executor with a solo model

# Goal
The advisor strategy claims that a cheaper executor with a stronger advisor reaches close to the quality of the strong
model alone at a fraction of its cost. For Cardinal's own work this is unproven. The outcome: `skill:evaluate-harness`
shows, for the same harness, the efficiency and the delivered quality of each setup (executor alone, executor with an
advisor, strong model alone), so the user can pick the default model of a project from numbers.

# Findings
* Anthropic's published guidance: run the same eval on Sonnet solo, Sonnet with an Opus advisor and Opus solo.
* The user's sessions mix Opus 5.5 (about 49,600 assistant messages) and Sonnet 5.5 (about 21,100), mostly with an
  Opus advisor configured (counted 2026-10-30).
* `cardinal-report` groups by Cardinal revision (REP-4) and only lists the models of a group (REP-5): it cannot split a
  group by setup.

# Approach
1. A setup per session: the main model plus the advisor model (from `count_the_advisor_calls_of_each_session.md`), or
   `solo`.
2. Candidate: a `--by setup` option of `cardinal-report` grouping by setup instead of revision, the other columns
   unchanged; or a setup column in the efficiency and quality tables.
3. `skill:evaluate-harness` presents it, with the sample size of each setup and its confounders (project mix, version).

# Specs
* `specs/cardinal-report.md`: new requirements for the setup grouping.

# Milestone
Advisor

# Resources
* `specs/cardinal-report.md`, `specs/delivered-quality.md`, `files/.claude/skills/evaluate-harness/SKILL.md`.
* https://claude.com/blog/the-advisor-strategy

# Hints
* A setup is a choice of the user, often per task difficulty: say that the comparison is observational, not an A/B.
* The issues of one setup may be harder than another's; the reference task set
  (`replay_a_reference_task_set_to_compare_two_harness_variants.md`) is the controlled version.
