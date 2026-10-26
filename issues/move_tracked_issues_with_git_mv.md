# Move tracked issues with git mv when closing them

# Goal
`close_issue.rs` checks whether the issue is tracked with `git ls-files --error-unmatch -- <arg>` run with
`current_dir(dir)`, the `issues/` directory, while `<arg>` (`issues/foo.md`) is relative to the caller. From the
repository root git looks for `issues/issues/foo.md`, so every issue counts as untracked and is moved by a plain
rename: the commit then needs a manual `git add` of the old and new paths. Tracked issues should be moved with
`git mv`, so `git status` shows the rename staged.

# Findings
* 2026-10-26, `just close-issue issues/keep_the_queue_tidy_when_its_last_issue_closes.md` from the repo root left
  ` D issues/<name>.md` and `?? issues/done/<name>.md` instead of a staged rename; `git ls-files --error-unmatch --
  issues/done/<name>.md` run in `issues/` fails with `pathspec ... did not match`.
* The test `tracked issue is moved` of `test_close_issue.rs` only checks the file is at its target, so it passes
  with the plain rename.

# Approach
1. Run the `git ls-files` check with the caller's current directory (drop `current_dir(dir)`), or pass the file
   name instead of the path; check `git mv` gets the same paths.
2. Make `tracked issue is moved` also assert that `git status --porcelain` shows a staged rename (`R `), and the
   untracked case stays a plain rename.

# Resources
* `files/.claude/skills/issue-workflow/close_issue.rs` (`tracked`, `moved` in `main`)
* `files/.claude/skills/issue-workflow/test_close_issue.rs`

# Hints
* Change `files/`, then `just sync`; `just test-harness` runs the script tests.
