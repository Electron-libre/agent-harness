# Handle squash and rebase merged issue branches in finish-issue and next-issue

# Goal
An issue branch merged by squash or rebase must be cleaned by `just finish-issue`, and a stale branch must not keep an
issue out of `just next-issue` without the user seeing why.

# Findings
* `issue_worktree.rs` `finish` requires `git merge-base --is-ancestor <stem> <trunk>`, which is false after a squash or
  rebase merge. It errors "not merged" and removes nothing.
* `next_issue.rs` marks an issue as taken when a local branch has the issue stem as name. A leftover branch (squash
  merge, `git worktree remove` without `branch -d`) keeps the issue taken, and the output can be "every issue of the
  queue is taken" with only the Taken lines to explain it. A branch named like a trunk (`main`) would match a stem too.

# Approach
1. Add failing tests for both: a squash-merged branch, a leftover branch with no worktree.
2. Candidates: detect a merge by patch equivalence (`git cherry`) or by the merged pull request, or accept a
   `--force` flag after a prompt; count a branch as taking the issue only when it has a worktree or commits not on trunk.

# Specs
* `specs/parallel-issues.md`: amend the requirements for finish and for taken issues.

# Resources
* `files/.claude/skills/issue-workflow/issue_worktree.rs`, `files/.claude/skills/issue-workflow/next_issue.rs`, `specs/parallel-issues.md`

# Hints
* `finish` must never delete work that is not on trunk.
