# Report the evolution of the harness by revision

# Goal
On demand, tell whether a harness revision made the agent more efficient, the work better and the rules better
followed, compared to the previous ones, with the confounders shown next to the numbers. Also show what the harness
costs in every session, and which skills and hooks earn their place.

# Findings
* 2026-10-26, skill calls over all the transcripts: `code-review` 272, `issue-workflow` 98, `commit-message` 78,
  `create-issue` 46, `update-docs` 32, `write-spec` 21, `prioritize-issues` 1, `audit-instructions` 0.
* Model mix of the assistant turns: 15629 `claude-sonnet-5-5`, 2298 `claude-opus-5-5`, 1550 `claude-sonnet-5`;
  10 Claude Code versions in 30 days. A revision compared to another often differs by model and version too.
* The fixed context of the harness (`CLAUDE.md`, `AGENTS.md`, skill descriptions, hook outputs) is paid in every
  session; `CLAUDE.md` and `AGENTS.md` alone are 4.4 kB here.

# Approach
1. A skill (`evaluate-harness`, name to confirm) that runs the extractors of the issues below and writes a report
   per harness revision: efficiency (tokens, turns, user interventions per issue), quality, rule breaches, hook
   refusals and their false positives, skill usage (unused skills, skills called where another fits), fixed
   context cost of the revision.
2. Show next to each number the model, Claude Code version and project mix, and the sample size; refuse to
   conclude below a threshold (to pick).
3. Optionally commit an aggregate snapshot per revision (numbers only, no project name, no message text) so the
   history outlives the transcripts: to decide.
4. Report to the user as `AGENTS.md` section "Reports to the user" says (a table per axis, the trend only).

# Resources
* `issues/extract_session_metrics_from_the_transcripts.md`, `issues/detect_rule_breaches_in_the_transcripts.md`,
  `issues/measure_the_delivered_quality_from_the_git_history.md`
* `files/.claude/skills/audit-instructions/SKILL.md` (sibling: it proposes rules, this one measures them)
* `files/AGENTS.md`, section "Reports to the user"; `.claude/harness.toml` (`confidential`)

# Hints
* Numbers become targets once shown: report trends, never pass/fail thresholds on agent behaviour.
* The report must hold nothing specific to one consumer when it lands in a shared file.
