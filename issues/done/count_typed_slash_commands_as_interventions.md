# Count the slash commands the user types as interventions

# Goal
`session_metrics.rs` splits the typed messages into plain validations and the rest, through the typed-message filter
it shares with `audit_sources.rs`. That filter drops every line wrapped in `<…>`, so a slash command the user types
(`<command-name>/x</command-name>…`) counts in neither column: a session steered by slash commands looks like it
needed fewer interventions. Decide whether they count, and where.

# Findings
* 2026-10-26, `/code-review high` of `extract_session_metrics_from_the_transcripts`: the finding, out of the
  requirements of EVA-4 (which reuses the audit filter).
* 2026-10-26, all the transcripts: 5 typed slash commands in the main sessions (`/update-docs` 2, `/model` 2,
  `/compact` 1), next to 1433 typed messages: the skew is small today.

# Approach
1. Measure again over a longer window before changing anything: the count may stay negligible.
2. Candidates: a `slash_commands` column (by command name, like `skills`); or count them in `other_messages`; the
   audit filter itself stays as is (a slash command is not an instruction to audit).

# Specs
* `specs/evaluation.md`: a new requirement, or an amendment of EVA-4.

# Resources
* `files/.claude/transcripts/transcripts.rs` (`typed_text`), `files/.claude/transcripts/session_metrics.rs`
* `specs/evaluation.md`

# Hints
* A change of `transcripts.rs` needs the new SHA-256 line in both scripts (EVA-13).

# Tracking

Plan: validated

## Understanding
Decide whether the typed slash commands count as interventions, from a measure first. The measure over every
transcript (2026-09-09 to 2026-10-26, 202 sessions) finds the same 5 slash commands (`/update-docs` 2, `/model` 2,
`/compact` 1) next to 1469 typed messages (0.3 %), one per session at most; `/model` and `/compact` steer the
session, not the work. Out of scope: the audit filter (it stays as is).

## Plan
1. [x] Record the measure and the decision (no column now; revisit when the typed slash commands pass 2 % of the
   typed messages of a `harness_report` window) in this issue, then close it; verified by the measure above.

## Decisions
- Mode: autonomous
- Base commit: `c62ab8a87cb565e9cc3a4219e4573ed405485f55`
- 2026-10-26: no change: the typed slash commands stay out of both columns (0.3 % of the typed messages, the same 5
  as the first finding); reopen when they pass 2 % of the typed messages of a `harness_report` window, with a
  `slash_commands` column by command name (like `skills`) as the first candidate.

## Summary of deliverable 1
done: measure and decision recorded; no code, spec or doc changed.

| check | result |
|---|---|
| tests, lint, specs | not run: no code changed |
| docs | none to update |
| code review | low on `issues/`: no findings |

red: none (no test)

Code review: no findings

Status: done, closed.
