# Keep the session transcripts long enough to evaluate the harness

# Goal
Evaluating the harness from the session transcripts needs a history that spans several harness revisions.
Claude Code deletes the transcripts after a retention period, so the history is lost before it can be compared.
Make sure the transcripts (or what the evaluation needs from them) survive long enough.

# Findings
* 2026-10-26: 188 session transcripts across the projects directory of Claude Code; the oldest one dates from
  2026-09-26, exactly 30 days earlier, which matches the default retention (`cleanupPeriodDays`, 30 days).
* No `cleanupPeriodDays` is set in the user settings nor in `files/.claude/settings.json`.
* The transcripts hold private content: `skill:audit-instructions` already forbids writing it in a repository.

# Approach
Measure first: confirm from the Claude Code docs where `cleanupPeriodDays` is read (user settings only, or
project settings too) and whether a project value changes the cleanup of every project.
1. Document in `README.md` that the evaluation needs a longer retention, with the setting to raise in the user
   settings (the harness does not own them).
2. Have `harness check` (or `sync`, like the settings warning of `issues/done/warn_of_the_settings_at_sync.md`)
   warn when the retention is the default.
3. Archive, outside the repositories, a compact extract of each session (metrics only, no message text) so the
   history no longer depends on the retention; to combine with the metrics extractor of
   `extract_session_metrics_from_the_transcripts`.

# Resources
* `files/.claude/skills/audit-instructions/SKILL.md` (confidentiality of the transcripts)
* `issues/done/warn_of_the_settings_at_sync.md`
* Claude Code settings documentation (`cleanupPeriodDays`)

# Hints
* Urgent by nature: every day without it loses one day of history.
* The harness must not rewrite the user settings; warn or document only.
