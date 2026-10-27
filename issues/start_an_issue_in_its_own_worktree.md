# Start an issue in its own worktree

# Goal
The worktree isolation of ADR 0003 is never used: an agent handed an issue runs `issue-workflow` in the main worktree,
so two sessions share files, index and `target/`, and `just next-issue` still offers the issue as free. Make starting
an issue in its own worktree the default path of `issue-workflow`, and make the hooks act on the worktree the agent
works in.

# Findings
* Transcripts of the user's projects (2026-10-27): 188 sessions name `issue-workflow` across 5 projects; no session
  ran `just start-issue` as a command, and no Claude Code project directory exists for an issue worktree
  (`<repo>-<stem>`), so no session was ever opened in one.
* Observed on 2026-10-27: a session drove a newly created issue in the main worktree; its untracked draft spec made
  the review gate refuse another session's commit of `issues/` files only (the snapshot covers untracked files).
* `issue-workflow` `SKILL.md`: step 0 never mentions the worktree; the section "Parallel agents" says two agents
  "can" work in worktrees and sends to `details.md`, itself titled "the rare cases". `details.md` expects the user to
  run `just start-issue` and to "open the agent there".
* `start-issue` refuses an issue that is not committed on the main branch (PAR-2): an issue just written by
  `create-issue` and handed over at once cannot get a worktree.
* `format_rust.rs` and `clippy_gate.rs` run `cargo fmt` and `cargo clippy` in `CLAUDE_PROJECT_DIR`, the directory
  the session was opened in: an agent that moves into an issue worktree would format and lint the main worktree,
  where another agent works. `require_code_review.rs record` uses the `cwd` of the event, so it follows the agent.

# Approach
Measure first: in a real session, check what Claude Code passes to the hooks (`cwd`, `CLAUDE_PROJECT_DIR`) after
the agent changes directory, and whether its built-in worktree tool (`EnterWorktree`) can enter an existing
directory or only creates its own.
1. Step 0 of `issue-workflow`: in the main worktree, when the issue is not taken, run `just start-issue` and do all
   the work in the worktree (every command run from it); when the issue is uncommitted, ask the user to commit it
   first, or commit it alone.
2. The hooks resolve the work tree from the `cwd` of the event, as `record` does, rather than from
   `CLAUDE_PROJECT_DIR`.
3. Or keep the user's launch in the worktree, but make `issue-workflow` refuse to start in the main worktree when
   another issue is taken or the tree is dirty with files of another issue, telling the user the command to run.

# Specs
* `specs/parallel-issues.md`: requirements for the start of an issue and for the directory the hooks act on;
  `specs/clippy-gate.md` for the gate's directory.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (step 0, "Parallel agents"), `files/.claude/skills/issue-workflow/details.md`
* `files/.claude/skills/issue-workflow/issue_worktree.rs`, `files/.claude/hooks/format_rust.rs`, `files/.claude/hooks/clippy_gate.rs`
* `docs/adr/0003-one-worktree-per-issue.md`

# Hints
* The worktree has its own `.claude/` copy (tracked), so hooks invoked with `CLAUDE_PROJECT_DIR` still run the main
  worktree's scripts: the same revision only while both are on the same commit of `files/`.
* A session opened in the worktree gets its own Claude Code project directory, which the transcript readers must map
  back to the repository (`transcripts.rs` resolves a project from its directory).
