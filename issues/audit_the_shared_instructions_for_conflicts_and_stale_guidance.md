---
rank: 150
why: 'after the routing count, which shows whether an audit fix changed anything'
---
# Audit the shared instructions for conflicts and stale guidance

# Goal
The managed instructions grew one issue at a time: `CLAUDE.md` templates, the always-on rule, eight skills of which
`issue-workflow` alone is 171 lines. Rules written for older models (emphatic "never", repeated warnings, steps the model
now does unprompted) and rules that contradict each other cost fixed context and make the agent hesitate. The outcome:
a reviewed list of the contradictions, duplicates and stale rules of `files/`, and the fixes the user accepts, each
measured.

# Findings
* `files/.claude/skills/*/SKILL.md` total 579 lines, `issue-workflow` 171 plus its `details.md`; one always-on rule of
  8 lines (counted 2026-10-30).
* `skill:audit-instructions` proposes new skills and hooks from what the user typed; it does not review the existing
  instructions for conflicts or staleness.
* The article on folding subagents names the "Frankenprompt" (contradictory rules piled up) and points to a prompt audit
  that flags instructions written for older models and proposes diffs; it also puts the mechanical parts in code.
* Candidate mechanical rule now in prose: the size limit of a deliverable (`git diff --shortstat` against ~1,000 lines),
  which a `cardinal run` tool could compute.

# Approach
1. Read `files/` whole and list, with file and line: contradictions, duplicates across skills, rules a current model
   follows unprompted, rules that a tool could check.
2. Show the list for validation; one commit per accepted family of fixes, with `Expected-effect:` on the fixed context
   and the metric it should not hurt.
Candidate: make the review a mode of `skill:audit-instructions` rather than a one-off.

# Milestone
Lean agent loop

# Resources
* `files/`, `files/.claude/skills/audit-instructions/SKILL.md`, `specs/instruction-audit.md`.
* https://claude.com/fr/resources/articles/building-agents-when-to-fold-subagents-into-your-main-agent-loop

# Hints
* A rule kept because of a past incident has its issue in `issues/done/`: read it before removing the rule.
* Cut one family at a time, so a regression points at its cause.
