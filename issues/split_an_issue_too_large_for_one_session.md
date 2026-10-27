# Split an issue too large for one session

# Goal
An issue that cannot be delivered in one session with diffs a `low` review reads whole costs far more than the sum
of its parts: context grows with each deliverable, reviews see part of the diff, and the `high` review comes too late.
Give `create-issue` and `prioritize-issues` a size limit beyond which an issue is split before it is started.

# Findings
* `ship_the_harness_scripts_as_prebuilt_binaries` (closed 2026-10-27): 7 planned deliverables plus one of fixes,
  about 7,500 lines of scripts and tests moved, the launcher, a release workflow and CI, in one session.
* The `low` review saw only part of a 73 kB diff (deliverable 4) and was run twice when new files were untracked;
  the parts it missed were compared by hand.
* The final `high` review found 10 problems over the whole issue, which became an extra deliverable; several were in
  deliverable 1 and would have been cheaper to fix there.
* Natural cut points existed: launcher and release, hooks, issue tools and checkers, transcript readers, cleanup.

# Approach
1. Measure on `issues/done/`: deliverables per issue, diff size per deliverable commit, and the reviews whose report
   says they saw part of the diff (`harness run delivered-quality` gives deliverables and review findings).
2. Pick a limit from that (deliverables, or an estimated diff size), and where it is checked: `create-issue` when the
   issue is written, `issue-workflow` step 1 when the plan exceeds it (propose the split at the step 2 stop),
   `prioritize-issues` for the open ones.
3. Define how a split issue is written: one issue per cut, ordered in the queue, the dependencies in `# Hints`.

# Resources
* `files/.claude/skills/create-issue/SKILL.md`, `files/.claude/skills/prioritize-issues/SKILL.md`,
  `files/.claude/skills/issue-workflow/SKILL.md` (steps 1 and 2)
* `issues/done/ship_the_harness_scripts_as_prebuilt_binaries.md` (its `# Tracking`)

# Hints
* A limit in deliverables alone misses a deliverable that moves thousands of lines; the review coverage is the cost
  that matters.
