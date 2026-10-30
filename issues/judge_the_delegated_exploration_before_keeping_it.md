---
rank: 75
why: 'cheap and immediate: the delegation already ran for many sessions; decides whether the one subagent of the workflow stays'
---
# Judge the delegated exploration before keeping it

# Goal
`skill:issue-workflow` hands a broad exploration to an `Explore` subagent to keep the main context small. A subagent is
worth its cost only when the single loop degrades measurably from a cluttered context: each one starts cold, re-reads
what the main agent may already know, and hands back a summary that loses detail. The outcome: a verdict on the
delegation from the sessions since it shipped, on all the metrics it may move, and a decision to keep, narrow or fold
it back into the main loop.

# Findings
* The delegation shipped with `Expected-effect: main_context_peak down` only; `input_tokens` was left out on purpose
  ("its direction is unknown: each subagent starts cold"), and no quality effect was declared
  (`issues/done/delegate_the_exploration_of_the_issue_workflow_to_a_subagent.md`).
* Its own tracking left "not verified: whether the agent delegates in practice".
* The article on folding subagents into the main loop reports a customer about 2x faster with a single loop, and keeps
  subagents for three cases only: measured context degradation, parallel work on fresh contexts, tool confusion.

# Approach
1. Run `skill:evaluate-harness` over the revisions around the delegation commit: `main_context_peak` and
   `peak_sessions`, `input_tokens`, `turns`, the duration, and the quality table.
2. Count how often the agent actually delegates during an issue (the `Agent` calls with `Explore` per issue session).
3. Decide with the user: keep, narrow (e.g. history searches only) or fold back (remove the paragraph, commit with the
   opposite `Expected-effect:`).

# Milestone
Lean agent loop

# Resources
* `issues/done/delegate_the_exploration_of_the_issue_workflow_to_a_subagent.md`, `specs/cardinal-verdicts.md`.
* `files/.claude/skills/issue-workflow/SKILL.md`, step 1 "Delegate a broad exploration".
* https://claude.com/fr/resources/articles/building-agents-when-to-fold-subagents-into-your-main-agent-loop

# Hints
* A verdict on few sessions is `confounded` (REP-10): say the sample before any decision.
* Look at which issues changed outcome, not only the medians.
