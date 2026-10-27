# Create the issue worktrees under .claude/worktrees

# Goal
Entering the worktree of an issue costs the user one approval per issue: `issue-workflow` step 0 calls
`EnterWorktree` on `../<repo>-<stem>`, and Claude Code asks for every path outside the repository's
`.claude/worktrees/`, whatever the permission rules say. Create the issue worktrees at `.claude/worktrees/<stem>`
instead, so that starting or resuming an issue needs no interaction.

# Findings
* Transcript of an issue session of this repository (2026-10-27): the only tool call approved by hand is
  `EnterWorktree` on `../agent-harness-<stem>`, recorded with `permissionDecision` `source: user_temporary`,
  `reasonCode: safetyCheck`; `just start-issue` was accepted by the `Bash(just *)` rule of the settings.
* Claude Code documentation, page "worktrees" (read 2026-10-27): "When Claude enters a path outside the repository's
  `.claude/worktrees/` directory, Claude Code asks for your approval first [...]. An `EnterWorktree` permission rule
  or choosing "don't ask again" doesn't suppress this prompt; only `bypassPermissions` mode skips it."
* The sibling path is fixed by ADR 0003 ("next to the main one (`../<repo>-<stem>`)"), by PAR-1 of
  `specs/parallel-issues.md` ("outside the repository, in the sibling directory `<repo>-<stem>`"), and is computed by
  `sibling()` in `files/.claude/skills/issue-workflow/issue_worktree.rs`. It is also written in
  `files/harness.just` (`start-issue` comment), `files/.claude/skills/issue-workflow/SKILL.md` (step 0) and
  `docs/architecture.md`.
* `check_specs.rs` already skips `.claude/worktrees` when it collects the test files; `close_issue.rs` skips
  `skipped_dirs` of `.claude/harness.toml`. Other scanners of the repository tree were not reviewed.
* This repository's `.gitignore` does not ignore `.claude/worktrees/`, and it un-ignores `.claude/`: an issue
  worktree there would show as untracked in the main worktree.

# Approach
1. `issue_worktree.rs start` creates `<main worktree>/.claude/worktrees/<stem>` (creating the parent directory) and
   prints that path; `finish` and the taken detection follow, since they read the path from `git worktree list`.
2. Ignore `.claude/worktrees/` in every consumer: either `start-issue` writes it to `.git/info/exclude` (nothing to
   sync, works in any project), or the harness ships or checks a `.gitignore` line. Compare both.
3. List every script that walks the repository tree (spec checker, close-issue, transcript readers, `harness
   check`/`sync`, the hooks) and make each skip `.claude/worktrees`, so that a main worktree scan never reads the
   issue worktrees nested in it.
4. Update step 0 of `issue-workflow`, the recipe comments, `docs/architecture.md`, and amend ADR 0003 (or supersede
   it with a new ADR) for the new location.
5. Existing sibling worktrees: keep supporting them until their `finish-issue` (resume by path, with the approval),
   or document a manual move with `git worktree move`.

# Specs
* `specs/parallel-issues.md`: amends PAR-1 (location of the worktree); possibly a new requirement for the ignore
  rule and for the scanners skipping `.claude/worktrees`.

# Resources
* `docs/adr/0003-one-worktree-per-issue.md`, `specs/parallel-issues.md`.
* `files/.claude/skills/issue-workflow/issue_worktree.rs`, `SKILL.md` step 0, `details.md`.
* `files/.claude/specs/check_specs.rs` (existing `.claude/worktrees` skip) and its test.
* Claude Code docs: https://code.claude.com/docs/en/worktrees ("Ask Claude to create a worktree", "How Claude Code
  enforces isolation").

# Hints
* A nested worktree holds its own `Cargo.toml` workspace root, so cargo run inside it stops there; check that
  cargo run in the main worktree does not pick up the nested crates (it should not, the workspace members are
  listed explicitly, but verify).
* The hooks receive `CLAUDE_PROJECT_DIR` of the main worktree and the `cwd` of the event (see PAR-12): with the
  worktree nested in the main one, a check "is this path under the project dir" no longer tells the two apart.
* Do not reach for `bypassPermissions`: it removes every prompt, not only this one.
