# Keep the queue tidy when its last issue closes

# Goal
`close_issue.rs` drops the line of the closed issue from `issues/queue.md` and writes the kept lines joined by `\n`
plus one `\n`. When that line was the last entry, the blank line that separates the header from the list stays, so
the queue ends with `\n\n`: an empty queue then differs from a freshly written one. The queue should end with
exactly one newline whatever entry is removed.

# Findings
* 2026-10-26, `/code-review high` of `count_typed_slash_commands_as_interventions`: closing the last open issue
  left `issues/queue.md` ending with `\n\n`.

# Approach
1. Trim the trailing blank lines of the kept lines before writing them back, then add one `\n`.
2. A script test in `test_close_issue.rs` (or the test that already covers `close_issue.rs`): closing the last
   entry leaves the header ending with one newline.

# Resources
* `files/.claude/skills/issue-workflow/close_issue.rs` (the queue rewrite in `main`)
* `issues/queue.md` (line format)

# Hints
* Change `files/`, then `just sync`; `just test-harness` runs the script tests.
