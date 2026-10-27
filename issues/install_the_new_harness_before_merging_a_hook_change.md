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

# Specs
* `specs/tool-binaries.md` (BIN-15, BIN-16)

# Tracking

Plan: validated

## Understanding
Stop a hook change from reaching a project whose installed `harness` cannot parse the new hook commands: detect it
before the fast-forward (this repository) and before `update` writes (consumers). Out of scope: the words after the
subcommand (the pinned binary runs them), installed launchers older than the check. Means: approaches 1 + 2 + 3 of the
issue, the check being `harness help <subcommand>` run against the `harness` on the path.

## Plan
1. [x] Launcher check in `check`: a hook command `harness <sub> ...` of `.claude/settings.json` whose `<sub>` the
   `harness` on the path refuses fails `check`; verified by tests with a fake `harness`; makes true BIN-15.
2. [ ] `update` refuses such a target, naming `cargo install --git <source> --rev <commit> --locked`, and warns under
   `--dry-run`; verified by tests with a fake `harness`; makes true BIN-16.
3. [ ] Procedure and docs: `issue-workflow` "Merge back" runs `just check-harness` in the worktree before the
   fast-forward and installs the branch's `harness` (`just install` from the worktree) when it fails; the README says a
   consumer installs the matching `harness` when `update` refuses; verified by `just check-harness` and a reread.

## Decisions
- Mode: autonomous
- Base commit: 61cd2097e77756fcd581ca17d7de3d628cbcd552
- Questions: no override of the refusal of `update` (install, then run it again); `sync` not checked.

## Deliverable 1
done: `harness check` fails on each hook command `harness <sub>` whose `<sub>` the `harness` on the path refuses
(`harness help <sub>`, asked once per subcommand); no `harness` on the path, nothing looked at.

| check | result |
|---|---|
| tests | pass |
| lint | pass |
| specs | pass (BIN-15 traced, spec `draft` until BIN-16) |
| docs | README, `docs/architecture.md`, `check` help |
| code review | low, see below |

red: `assertion left == right failed: [] left: 0 right: 2` (no hook command refused before the code)

Code review: no findings

not verified: a real old launcher (one without `run`) against these settings; the fake `harness` stands for it.
