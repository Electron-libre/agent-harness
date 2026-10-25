# Make the agent reports concise and visual

# Goal
The skills make the agent end its work with long prose reports. Replace them with short, scannable output (tables,
checklists, one line per fact) and remove the other noise the skills ask the agent to produce, so the user reads
the state of the work at a glance.

# Findings
* Several skills already ask for a table: `prioritize-issues` step 5, `audit-instructions` step 4,
  `update-docs` step 6.
* Prose summaries are still requested elsewhere, with no format:
  * `issue-workflow` step 6 (human review summary: done, verified, not verified, decisions, `Code review:` line),
    step 0 (summary of where a resumed issue stands) and the final summary after the last deliverable.
  * `create-issue` step 6 ("a two-line summary").
  * `commit-message` and `write-spec` give no output format.
* Other noise to measure: restating the issue in step 1, repeating the plan, narrating each step.
* The size of real reports has not been measured yet.

# Approach
Measure first: read recent session transcripts for the final reports (length, repeated content) to see which
ones are the longest and what the user actually acts on.
Then, candidates that can be combined:
1. A shared output rule in `AGENTS.md` (or one small shared document the skills cite): facts in a table or
   checklist, no narration of steps, no repetition of what the diff or the issue already shows, prose only for
   a decision the user must take.
2. A fixed template for the step 6 review stop: a table `check | result` (tests, lint, specs, docs, review
   findings) plus a short list `not verified` and `decisions needed`.
3. The same treatment for the step 0 resume summary, the final summary, and `create-issue` step 6.
4. Drop instructions that only produce narration (restating the issue when it is already clear, announcing
   each step).

# Resources
* `files/.claude/skills/*/SKILL.md`, `files/AGENTS.md`
* `skill:prioritize-issues` and `skill:update-docs`: existing table formats to reuse
* Session transcripts of the project (for the measure)

# Hints
* Keep every validation stop and what it must show (`what was not verified`, the `Code review:` line): the
  hook `require_code_review.rs` and the process rely on them. Shorten the form, not the content.
* The conversation language stays the `user_language` of `.claude/harness.toml`; table headers follow it.
* Related issue: `drop_the_thread_rename_instructions` touches the same `issue-workflow` step 0; do it first.
