# Measure the delivered quality from the git history

# Goal
The cost of a session says nothing of what it delivered. Measure the quality of the delivered work by what
happened to it afterwards: fixes, reverts, review findings, failed checks, rework asked by the user.

# Findings
* 2026-10-26, `git log` of this repository: `3b72198 feat(files): prepare an issue in its files...` is followed by
  `c56fa40 fix(files): close the gaps of the single issue validation stop` on the same subject.
* Issue files under `issues/done/` record per deliverable the code review level and what it found, and the checks
  run (`# Tracking`, "Summary of deliverable n").
* The commit scopes are constrained (`commit_scopes` of `.claude/harness.toml`, `just check-commit`), so a fix and
  the feature it fixes can be matched by scope and files touched.

# Approach
1. From git: per issue, the `fix` and `revert` commits touching the files of its `feat` commits within a window
   (to pick), the number of commits per deliverable.
2. From the issue files: the review findings and fixes recorded per deliverable.
3. From the transcripts (with `extract_session_metrics_from_the_transcripts`): failed `just check` runs before a
   commit, and the user's answers at the validation stops that ask for rework.
4. Report per issue and per harness revision; leave the interpretation to the report of
   `report_the_harness_evolution_by_revision`.

# Specs
* `specs/evaluation.md`, new requirements for the quality signals.

# Resources
* `AGENTS.md` (commit format), `issues/done/`, `issues/queue.md`
* `files/.claude/skills/issue-workflow/SKILL.md` (what `# Tracking` records)
* `issues/extract_session_metrics_from_the_transcripts.md`

# Hints
* A fix after a feature may come from a new need, not a defect: report it as a signal, not a verdict.
* Consumers have their own scopes and history; read their git log, never copy it into this repository.
