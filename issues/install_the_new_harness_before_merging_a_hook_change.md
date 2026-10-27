# Install the new harness before merging a change of the hook commands

# Goal
When an issue changes the commands the hooks run (a new `harness run` tool, a new subcommand of `harness`), the hooks
of the main worktree switch at the fast-forward of the merge. If the installed `harness` does not know the new
command, every hook fails and the agent is locked out. The merge procedure must install the new binary first.

# Findings
* Closing `ship_the_harness_scripts_as_prebuilt_binaries` (2026-10-27): after `git merge --ff-only`, the PreToolUse
  hook `harness run review-gate check` exited 2 (`unrecognized subcommand 'run'`), so Claude Code refused every Bash
  command, `just install` included; the Stop hook `harness run clippy-gate` refused each end of turn, so the agent
  repeated the same request for four turns. The user had to run the install by hand.
* The issue had noted the need for `just install`, but placed it after the fast-forward.
* `harness run` now exits 3 when it cannot get a tool (BIN-14), but an installed binary without `run` exits 2 through
  its argument parser, which no change of the new code can prevent.

# Approach
1. In `issue-workflow` "Merge back" (`details.md`): when the branch changes `entries/.claude/settings.json` or the
   `harness` crate, install the binary of the branch (`cargo install --path crates/harness --locked` from the
   worktree, or the recipe) before the fast-forward.
2. Or detect it: a check run before the merge that compares the commands of the managed hooks with what the installed
   `harness` accepts, and stops with the command to run.
3. For consumers: say in the README that a project moving to a revision with new hook commands needs the matching
   `harness` installed first, or have `harness update` refuse when the installed binary is older than the target.

# Resources
* `files/.claude/skills/issue-workflow/details.md` ("Merge back"), `entries/.claude/settings.json`
* `specs/tool-binaries.md` (BIN-14), `crates/harness/src/main.rs` (`run`)

# Hints
* The installed `harness` is the launcher of every project of the machine; an install is not reversible by the agent
  once the hooks refuse its commands.
