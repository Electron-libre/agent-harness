# Work an issue: details

The cases `SKILL.md` leaves out, read when one of them comes up.

## Review waiver

Only the user can waive the review of step 3.5, with `just waive-review`: it creates the waiver of the current
worktree (also in a linked worktree, where the waiver is per worktree). Never create that file nor run that recipe
yourself. Deny rules of `.claude/settings.json` refuse the recipe and an edit of the file; naming them (commit
message, `grep`) is fine.

## Worktree of an issue

Step 0 of `SKILL.md` puts each issue in its own worktree: `just start-issue issues/<name>.md` creates
`../<repo>-<name>` on a branch `<name>`, and the session enters it with `EnterWorktree` (`path`). A shell `cd` is no
substitute: the Bash tool resets the directory after a command that left the launch directory. Once entered, the
`cwd` of the hook events is the worktree, and the hooks act on it (`clippy_gate.rs`, `require_code_review.rs`; `format_rust.rs` formats
the project of the edited file); their scripts still come from the main worktree's `.claude/` (`CLAUDE_PROJECT_DIR`),
which holds the same revision while both worktrees are on the same commit of the harness. While the branch or
worktree exists, `just next-issue` skips the issue. `start-issue` refuses an issue not committed on the main
branch, hence the commit of the issue (with its queue line) before it. The user may also open a session in the worktree directly;
step 0 then has nothing to do.

## Merge back

Once the user agreed to close the issue:

1. In the worktree, commit the last tracking, then `git rebase <main branch>` (the branch checked out by the main
   worktree); a conflict stops here, for the user.
2. `ExitWorktree` with `keep`: the session is back in the main worktree.
3. There, `git merge --ff-only <name>`, then `just close-issue issues/<name>.md` and commit the move (`docs: close
   ...`); when `close-issue` rewrote documents outside `issues/`, that commit needs its `low` review like any diff.
4. `just finish-issue <name>` removes the worktree and the branch, once merged and clean.

`finish-issue` refuses inside the worktree it would remove (`--force` or not), and refuses a branch whose work is
not on the main branch: after the user agreed, `just finish-issue <name> --force` deletes it anyway. `close-issue`
refuses in a linked worktree and an issue of one named by path from the main one, so the queue is never edited on
an issue branch.
