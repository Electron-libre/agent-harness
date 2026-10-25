# Block Claude/Anthropic attribution in commit messages and pull request descriptions

# Goal
A hook that stops the agent from creating a commit or a pull request whose text carries a Claude or Anthropic
credit to the agent (a trailer naming it as co-author, a "generated with" footer). The rule is already written
down several times and still gets broken, because the harness default reminder asks for the opposite.

# Findings
* Found by the instruction audit (counts only): the rule exists in the memory of two projects, in the user's
  global `CLAUDE.md`, and as one explicit correction typed by the user; in one project the agent's name still
  showed up among the contributors of the hosting service.
* `commit_check.rs` already rejects the trailer, but only when the agent runs it; nothing stops a bare
  `git commit -m` or a `gh pr create --body`.
* `files/.claude/hooks/` holds `format_rust.rs` and `clippy_gate.rs`; hooks are declared in
  `.claude/settings.json`, which is seeded once (`seed/`), not managed: a new hook reaches the consumers only
  if the declaration is added to their file too.

# Approach
1. A `PreToolUse` hook on `Bash` that reads the command, and for `git commit` / `gh pr create|edit` looks for the
   attribution patterns in the message (inline, heredoc, `-F` file) and exits 2 with the reason so the agent
   rewrites it.
2. Reuse the patterns of `commit_check.rs` rather than a second list.
3. Decide how consumers get the declaration (documented snippet, or a `harness` step), at `issue-workflow`
   steps 1 to 3.

# Resources
* `files/.claude/hooks/`, `files/.claude/skills/commit-message/commit_check.rs`, `AGENTS.md`
* `seed/.claude/settings.json`, `docs/architecture.md`

# Hints
* The hook must not block a commit whose message merely discusses the rule (this repository's own history).
* The shared files hold nothing specific to one project.
