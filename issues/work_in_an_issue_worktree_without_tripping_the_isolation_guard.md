# Work in an issue worktree without tripping the isolation guard

# Goal
After `EnterWorktree`, Claude Code isolates the session in the worktree and refuses any Bash command it cannot prove
stays there. In the last issue this refused dozens of ordinary commands and forced detours. Find how `issue-workflow`
should enter or work in an issue worktree so the agent's usual commands go through.

# Findings
* Session of `ship_the_harness_scripts_as_prebuilt_binaries` (2026-10-27): refused with "too complex to verify that it
  stays inside the worktree" or "cannot be shown not to run git": heredocs feeding `python3` whose text named `git`,
  `for` loops running `sed` with a variable, a script under `/tmp` running `git -C`, `time ( ... )`, a pipe into the
  launcher, commands chaining `git add -N` with other commands (refused whole, so the following steps silently did not
  happen and a review ran without the new files).
* The workarounds were scripts written to a temporary directory and run with paths as arguments, one plain command
  per call; each refusal cost a call and its context.
* The guard is Claude Code's, not the harness's: nothing in this repository or the user settings produces it.

# Approach
1. Measure: count the refused calls per session of the issue worktrees (the transcripts hold the refusal text) with
   the transcript readers, and classify them by shape.
2. Candidates, to compare:
   1. Open the session in the worktree directly (a new session started there) instead of `EnterWorktree` from the
      main one, if the guard only applies to an entered worktree.
   2. Add to `issue-workflow` (or `AGENTS.md`) the command shapes that pass: one command per call, scripts in files
      with paths as arguments, no `cd` into another directory.
   3. Recipes for the multi-step operations that were refused (`git add -N` of new files before a review, a
      comparison of moved code).
3. Check that the review gate and the clippy gate still see the worktree with the chosen way in.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (step 0), `details.md`
* `docs/adr/0003-one-worktree-per-issue.md`, `docs/adr/0008-issue-worktrees-nest-under-claude-worktrees.md`
* `harness run rule-breaches`, `harness run session-metrics` (refusals of tool calls)

# Hints
* A command refused whole runs none of its parts: a chain of steps must not hide a refusal from the next step.
* The user favours candidate 2.1 (a session opened in the worktree) as the compromise. To check first: that the
  guard does not apply to such a session, and how step 0 hands over, since the agent cannot start a session itself
  (`just start-issue` prints the path, the user opens the session there).
