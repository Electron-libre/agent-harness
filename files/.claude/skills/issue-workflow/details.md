# Work an issue: details

The cases `SKILL.md` leaves out, read when one of them comes up.

## Review waiver

Only the user can waive the review of step 3.5, with `just waive-review`: it creates the waiver of the current
worktree (also in a linked worktree, where the waiver is per worktree). Never create that file nor run that recipe
yourself. Deny rules of `.claude/settings.json` refuse the recipe and an edit of the file; naming them (commit
message, `grep`) is fine.

## Worktree of an issue

Step 0 of `SKILL.md` puts each issue in its own worktree: `just start-issue issues/<name>.md` creates
`.claude/worktrees/<name>` on a branch `<name>`, and the session enters it with `EnterWorktree` (`path`), which asks no
approval there, unlike for a path outside `.claude/worktrees/` (an issue started before ADR 0008 at
`../<repo>-<name>`). `start-issue` also adds `/.claude/worktrees/` to `.git/info/exclude`. A shell `cd` is no
substitute: the Bash tool resets the directory after a command that left the launch directory. Once entered, the
`cwd` of the hook events is the worktree, and the hooks act on it (`clippy-gate`, `review-gate`; `format-rust` formats
the project of the edited file); `harness run` finds the `harness.toml` of the worktree and runs the tools
of the revision it pins (with a local source, those of the worktree's own crate). While the branch or
worktree exists, `just next-issue` skips the issue. `start-issue` refuses an issue not committed on the main
branch, hence the commit of the issue (with its queue line) before it. The user may also open a session in the worktree directly;
step 0 then has nothing to do.

## Merge back

Once the user agreed to close the issue:

1. In the worktree, commit the last tracking, then `git rebase <main branch>` (the branch checked out by the main
   worktree); a conflict stops here, for the user. Then run `just check-harness` in the worktree: when it names a
   hook the installed `harness` refuses (the branch adds a subcommand the hooks call), install the `harness` of the
   branch before going on: from the worktree, `cargo install --path crates/harness --locked` when its
   `.claude/harness.toml` has the local source `.` (the repository of the harness), else `cargo install --git <source>
   --rev <rev> --locked` with the `source` and `rev` of that file. After the fast-forward
   every hook goes through the installed `harness`, and one it refuses blocks every command, the install included.
2. `ExitWorktree` with `keep`: the session is back in the main worktree.
3. There, `git merge --ff-only <name>`, then `just close-issue issues/<name>.md` and commit the move (`docs: close
   ...`); when `close-issue` rewrote documents outside `issues/`, that commit needs its `low` review like any diff.
4. `just finish-issue <name>` removes the worktree and the branch, once merged and clean.

`finish-issue` refuses inside the worktree it would remove (`--force` or not), and refuses a branch whose work is
not on the main branch: after the user agreed, `just finish-issue <name> --force` deletes it anyway. `close-issue`
refuses in a linked worktree and an issue of one named by path from the main one, so the queue is never edited on
an issue branch.
