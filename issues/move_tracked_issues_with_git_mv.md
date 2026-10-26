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

# Tracking

Plan: validated

## Understanding
`close_issue.rs` must detect a tracked issue whatever the directory it runs from, so it moves it with `git mv` and
the rename is staged. Out of scope: a spec (no requirement of `specs/parallel-issues.md` covers the move itself).

## Plan
1. [x] Detect tracked issues from the caller's directory: run the `git ls-files` check without `current_dir(dir)`;
   verified by `tracked issue is moved` in `test_close_issue.rs` also asserting a staged rename (`R `) and the
   untracked case staying out of the index.

## Decisions
- Mode: autonomous
- Base commit: 3115a61e6b4348564a31499fd9a8f20d998a084d

## Deliverable 1

- done: the `git ls-files` check of `close_issue.rs` runs in the caller's directory, so a tracked issue is moved with
  `git mv`.

| check | result |
|---|---|
| tests (`just check`) | pass |
| `just test-harness` | pass |
| specs | no spec concerned |
| docs | no change needed |
| code review | `/code-review low` run |

- red: `FAIL - tracked issue is moved with git mv`
- Code review: no findings
- not verified: none
- decisions needed: none
- note: the fixture repo has no commit, so the test reads the index (`git ls-files`), not a `R ` status line.
