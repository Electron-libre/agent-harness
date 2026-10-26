# Refuse closing an issue of a linked worktree by path

# Goal
`close_issue.rs` checks for a linked worktree (PAR-8) in the caller's directory, not in the repository that holds
the issue. From the main worktree, `just close-issue ../repo-foo/issues/foo.md` passes the guard and edits the queue
and the references of the linked worktree, which PAR-8 forbids. The guard should check the worktree of the issue's
directory.

# Findings
* 2026-10-26, `/code-review high` of `move_tracked_issues_with_git_mv`: `linked_worktree()` calls
  `repo_root(Path::new("."))` and `git worktree list` in the current directory, before the issue path is read.

# Approach
1. Make `linked_worktree` take the directory to check: the issue's `issues/` directory for a close, `.` for
   `--relink`; validate the argument before the guard.
2. A case in `refuses_a_linked_worktree` of `test_close_issue.rs`: closing the linked worktree's issue by path from
   the main worktree is refused and leaves its queue untouched.

# Specs
* `specs/parallel-issues.md`, PAR-8 (its scenario may gain the by-path case).

# Resources
* `files/.claude/skills/issue-workflow/close_issue.rs` (`linked_worktree`, `main`)
* `files/.claude/skills/issue-workflow/test_close_issue.rs` (`refuses_a_linked_worktree`)

# Hints
* Change `files/`, then `just sync`; `just test-harness` runs the script tests.
