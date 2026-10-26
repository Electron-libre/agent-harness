# Tighten the issue workflow

# Goal
`issue-workflow` drives every change, so its gaps and its weight are paid on every issue. The review lets findings
outside the deliverable grow its scope, its cost depends on whatever level was typed last, the TDD loop has no
refactor step, and the skill loads rare details on each run. Make each issue cheaper and its deliverables smaller
and cleaner.

# Findings
* `files/.claude/skills/issue-workflow/SKILL.md` is about 10 KB, loaded for every issue; the "Parallel agents"
  section and the waiver details of step 4.5 serve rarely.
* Step 4.5 invokes `/code-review` without a level; the skill then reuses the level the user typed last.
* `issues/done/make_the_review_waiver_usable.md`: 10 review findings fixed inside one deliverable, none of them in
  the requirements it was planned for.
* Step 4 goes test, implement, docs, review: no step cleans the code once the test passes.
* Step 4.2 asks for a test that "fails for the right reason", but the summary of step 4.6 never shows the failure.
* Steps 1 and 3 are two separate stops even for an issue whose plan is one deliverable.
* Seven skills say "Show the user your results as `AGENTS.md` section ... says", which makes the agent read one more
  file to learn four rules.

# Approach
1. Review scope: a finding outside the requirements of the deliverable becomes an issue (`skill:create-issue`), not a
   fix in the deliverable; only findings on the deliverable's own change are fixed.
2. Review level: name it in step 4.5 (for example `low` per deliverable, `high` once at the end of the issue).
3. Add a refactor step after "Implement": clean under green tests, no new behaviour.
4. Show the red: one `red:` line with the failure message in the step 4.6 summary.
5. Let steps 1 and 3 be one stop when the issue changes no capability and fits one deliverable.
6. Move "Parallel agents" and the waiver details to `docs/architecture.md`, with one line pointing there; consider
   inlining the four report rules instead of citing `AGENTS.md`.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md`, `files/AGENTS.md` (section "Reports to the user")
* `files/.claude/skills/test_skills.rs` (checks the passages of `REQUIRED_PASSAGES` and the sections of `AGENTS.md`
  the skills cite)
* `docs/architecture.md`

# Hints
* `specs/agent-reports.md` and the wording checks of `test_skills.rs` are gone
  (`drop_redundant_guards_and_prose_tests`); `REQUIRED_PASSAGES` still pins the autonomous mode.
* Keep the validation stops that the autonomous mode never lifts.
