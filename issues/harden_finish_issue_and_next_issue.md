# Harden finish-issue and next-issue around merged and current worktrees

# Goal
`just finish-issue` must never remove the directory the user stands in, and `just next-issue` must free an issue
whose branch was squash-merged, whatever the number of commits.

# Findings (from a code review of the synced revision, to reproduce first)
* `finish` is documented as runnable from any worktree. Run inside the worktree of the issue, it removes the
  directory under the shell and later relative paths and `git` calls of that session fail.
* `next_issue.rs` `has_own_commits` is true as soon as `git cherry` shows a `+` line. A branch squash-merged from
  several commits keeps `+` lines, so the issue stays "taken" until `finish-issue --force`.

# Approach
1. Add failing tests for the two cases.
2. Candidates: refuse (or `cd` out first) when the cwd is inside the worktree to remove; detect a squash merge by
   comparing the tree diff of the branch against trunk.

# Specs
* `specs/parallel-issues.md`: amend the requirements for finish and taken issues.

# Resources
* `files/.claude/skills/issue-workflow/issue_worktree.rs`, `files/.claude/skills/issue-workflow/next_issue.rs`,
  `files/.claude/skills/issue-workflow/close_issue.rs`, `specs/parallel-issues.md`

# Hints
* `finish` must never delete work that is not on trunk.
