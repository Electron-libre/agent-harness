# Commit the issue that create-issue wrote

# Goal
`skill:create-issue` ends with "Don't commit unless asked", so after each new issue the user has to type "commit".
Let the skill commit the issue file and its queue line itself when that is all the work tree changed, and keep
asking otherwise.

# Findings
* `skill:audit-instructions` run of 2026-10-28 over the session transcripts: the user typed "commit" alone in 24
  sessions of 4 projects, up to 2026-10-28; in 21 of them the agent's previous message was about an issue and
  offered to commit it.
* `files/.claude/skills/create-issue/SKILL.md` step 6: "Don't commit unless asked (then `skill:commit-message`)".
  `skill:prioritize-issues` step 3 says the same for a reordered queue.
* The review gate never requires a review for a diff under `issues/` only (`review-gate.md`), so such a commit
  is not blocked.

# Approach
1. In `create-issue` step 6, commit through `skill:commit-message` when the staged change is only the new issue
   files and `issues/queue.md`; when other files are modified, commit only those issue files, or ask when the
   queue file holds changes that are not this skill's.
2. Decide at `issue-workflow` step 2 whether `prioritize-issues` gets the same rule.
3. Cover the passage in `crates/harness/tests/skills.rs` (`REQUIRED_PASSAGES`).

# Resources
* `files/.claude/skills/create-issue/SKILL.md`, `files/.claude/skills/prioritize-issues/SKILL.md`
* `files/.claude/skills/commit-message/SKILL.md`, `specs/review-gate.md`
* `crates/harness/tests/skills.rs`

# Hints
* Edit `files/`, then `just sync`.
* An issue created inside another deliverable (from a review finding) is committed with that deliverable: the
  rule must not commit it on its own then.
