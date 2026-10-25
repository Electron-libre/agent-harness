---
id: RPT
status: draft
adrs: []
---

# Agent reports

## Intent

The shared skills tell the agent what to show the user at each stop and at the end of its work. The user reads the
state of the work at a glance: facts in tables or checklists, prose only for a decision to take. The rule lives once
in `AGENTS.md`; the skills cite it and fix the layout of their own reports.

## Requirements

- **RPT-1** (deterministic) `AGENTS.md` must hold one section on the output of the agent, stating that facts go in a table or a checklist, that steps are not narrated, that what the diff, the issue or a spec already shows is not repeated, and that prose is kept for a decision the user must take.
- **RPT-2** (deterministic) Every skill that asks for a report to the user (`issue-workflow`, `create-issue`, `prioritize-issues`, `audit-instructions`, `update-docs`, `commit-message`, `write-spec`) must cite that section of `AGENTS.md` and must not restate its rules.
- **RPT-3** (deterministic) The human review of `issue-workflow` (step 4.6) must fix its layout: a table `check | result` covering tests, lint, specs, docs and code review, then the line `Code review:` with the findings and what was done about each (`no findings` when clean), then the lists `not verified` and `decisions needed`. None of these elements may be dropped.
- **RPT-4** (deterministic) The summary of a resumed issue (step 0), the final summary of `issue-workflow` and the delivery of `create-issue` (step 6) must each name a layout (table, checklist or fixed lines) instead of asking for a free summary.
- **RPT-5** (deterministic) The reformulation of `issue-workflow` (step 1) must name a short layout (goal, out of scope, questions) and must not ask to restate what the issue already states.
- **RPT-6** (deterministic) The skills must not ask the agent to announce or narrate its steps, nor to repeat the validated plan.
- **RPT-7** (deterministic) The hook `require_code_review.rs` must be unaffected: the markers and stops it relies on (`Code review:` line, `not verified`) stay in the skills.

## Scenarios

- (RPT-3) Given a deliverable whose review came back clean, when the agent presents step 4.6, then the table, the line `Code review: no findings` and the two lists are all present, a list being empty when nothing applies.
- (RPT-5) Given an issue that is already clear, when the agent reformulates it, then it states the goal, the out-of-scope and the questions only.

## Non-goals

- A numeric limit on the length of a report.
- The language of the reports: it stays the `user_language` of `.claude/harness.toml`, headers included.
- Changing which validation stops exist or what they must show.
