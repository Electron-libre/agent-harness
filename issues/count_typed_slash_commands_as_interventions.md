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
