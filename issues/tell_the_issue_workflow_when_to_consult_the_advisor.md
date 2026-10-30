---
rank: 50
why: 'after the advisor count, which gives it its verdict; the cheapest lever on the plan quality'
---
# Tell the issue workflow when to consult the advisor

# Goal
Claude Code has no setting to force or cap the advisor calls: only the instructions decide when the agent consults
it. `skill:issue-workflow` names no such moment, so a configured advisor is consulted at random or never. The outcome:
the skill names the few moments where advice pays (before committing to a plan, when stuck, before declaring done) and
how to weigh it, and stays neutral when no advisor is configured.

# Findings
* The advisor tool docs recommend a call before substantive work (after the orientation reads), when stuck or changing
  approach, and before declaring done once the deliverable is durable; and to follow the advice unless a step fails
  empirically or primary evidence contradicts it, then consult once more to reconcile.
* Anthropic measured on SWE-bench Multilingual +2.7 points and -11.9% cost per task for Sonnet with an Opus advisor
  against Sonnet alone.
* No advisor call is found in the user's transcripts though an advisor is configured (see
  `count_the_advisor_calls_of_each_session.md`).
* The workflow's matching moments: end of step 1 (plan written, before the stop of step 2), a check still red after a
  fix (step 3.5), before the `high` review of the whole issue.

# Approach
1. Add to `skill:issue-workflow` a short "Advisor" paragraph: the three moments above, the weighing rule, "skip it when
   no advisor tool is available".
2. Commit with `Expected-effect:` on the advisor call count (once it exists) and on `fixes_after` or
   `findings_per_review` down.
Candidate: the same paragraph as an always-on rule in `.claude/rules/` instead, if other skills need it; it costs fixed
context on every session.

# Milestone
Advisor

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 1, 2, 3.5, end of issue).
* https://platform.claude.com/docs/en/agents-and-tools/tool-use/advisor-tool (suggested system prompt for coding).
* `issues/count_the_advisor_calls_of_each_session.md`.

# Hints
* The advisor never replaces the `low` and `high` reviews nor the stop of step 2: they stay never lifted.
* Opus executors lost pass rate when nudged to call more: no "call it at least N times" rule.
