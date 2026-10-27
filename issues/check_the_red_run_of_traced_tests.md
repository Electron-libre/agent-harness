# Check the red run of the tests that trace a requirement

# Goal
A `// spec: ABC-n` citation proves that a test names a requirement, not that it fails without the code that meets it.
The `red:` line of `issue-workflow` step 3.6 records the failing run, but per deliverable, in prose, and nothing reads
it. Make the evidence that each traced test was seen failing checkable.

# Findings
* 19 issues of `issues/done/` hold a `red:` line, some `red: none`; it names test titles, not requirement ids.
* `check_specs.rs` counts citations; `delivered_quality.rs` reads the plan and `Code review` lines, not `red:`.

# Approach
1. Measure: over `issues/done/`, how many deliverables that traced a requirement recorded `red: none`.
2. Write the requirement ids in the `red:` line (`red: ABC-3, ABC-4: <failure>`), and have `delivered_quality.rs`
   report the traced ids with no red run.
3. Or a mutation check, heavier: revert the deliverable's non-test changes in a temporary worktree and require the
   cited tests to fail.

# Specs
* `specs/delivered-quality.md` if the measure reads the `red:` line.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 3.2 and 3.6), `files/.claude/transcripts/delivered_quality.rs`
* `files/.claude/specs/check_specs.rs`
