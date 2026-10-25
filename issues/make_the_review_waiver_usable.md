# Make the review waiver usable: mentions of the file and linked worktrees

# Goal
The user can waive the review of the next commit from any worktree, and the agent can still talk about the waiver
file (commit message, `grep`, docs) without being refused.

# Findings (from a code review of the synced revision, to reproduce first)
* `require_code_review.rs` refuses any Bash command whose text contains the waiver file name, with a raw substring
  match and no command parsing. A commit message that explains the waiver, or a `grep` for it, exits 2: the agent
  cannot commit or inspect the code and tests that document it.
* The instruction of `issue-workflow/SKILL.md` to waive the review is `touch .git/<waiver file>`. In a linked worktree
  made by `just start-issue`, `.git` is a file, so the command fails; the hook resolves the waiver with
  `git rev-parse --git-path`, which points into `.git/worktrees/<stem>/`. A waiver created in the main `.git` is ignored.
  The review marker is per worktree for the same reason.

# Approach
1. Add failing tests: a commit message and a `grep` that mention the file pass; the documented command creates a
   waiver the hook finds from a linked worktree.
2. Candidates: refuse only a command that writes the file (redirection, `touch`, `tee`, `cp`) after parsing like
   the commit checks do; document `touch "$(git rev-parse --git-path <name>)"`, or print the exact path in the refusal.

# Resources
* `files/.claude/hooks/require_code_review.rs` (the waiver check in `main`), `files/.claude/hooks/test_require_code_review.rs`,
  `files/.claude/skills/issue-workflow/SKILL.md` (step 4.5)

# Hints
* The hook refuses any Bash command naming the waiver file, so write the tests and this work without that literal
  in a shell command (use a heredoc-free editor tool or build the name in pieces).
* The deterrent must stay: the agent must not be able to waive its own review.
