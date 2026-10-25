# Run two code agents in parallel on different issues without polluting each other's work

# Goal
Let the user run two agents at once, each driving its own issue with `skill:issue-workflow`, without one
agent's edits, build artifacts, commits or queue changes leaking into the other's work. The isolation must
rely on git (a worktree or a branch per issue) and the merge back must stay a deliberate, validated step.

# Findings
* `issue-workflow` and `just next-issue` assume a single agent on one checkout: both agents would get the same
  "next" issue from `issues/queue.md`, and `just close-issue` edits the shared queue file.
* Two agents in one working tree share uncommitted files, the `target/` directory and the git index, so a commit
  or a formatter run by one captures or rewrites the other's changes.
* No measure yet of how often queue edits conflict when two branches close or create issues.

# Approach
Decide at `issue-workflow` steps 1 to 3; the candidates can be combined.

1. One `git worktree` per issue, on a branch named after the issue stem: separate working tree and index,
   shared object store. Compare with plain branches in a single checkout, which do not isolate uncommitted work.
2. A recipe or skill step that creates the worktree and branch for a given issue and removes them once the
   branch is merged, so the isolation is not left to the user's memory.
3. Mark an issue as taken so `just next-issue` skips it for the second agent, and decide where that mark lives
   (queue line, a file per worktree, or the branch itself) given that worktrees do not share uncommitted files.
4. Avoid queue conflicts: decide whether closing an issue edits `issues/queue.md` on the issue branch (merge
   conflicts possible) or at merge time.
5. Per-worktree build state: separate `target/` directories versus a shared `CARGO_TARGET_DIR`, trading disk and
   compile time against lock contention.

# Specs
* `specs/` entry for the isolation requirements if a `harness` subcommand or a managed script is added; none
  if it stays a skill step.

# Resources
* `files/.claude/skills/issue-workflow/` (`next_issue.rs`, `close_issue.rs`, `SKILL.md`)
* `issues/queue.md`, `docs/architecture.md`, `.claude/harness.toml`
* `files/.claude/hooks/` (hooks run per working tree; check they behave inside a worktree)

# Hints
* `.claude/harness.lock` and `.claude/harness.toml` are tracked, so each worktree has its own copy: `just sync`
  in one must not be needed in the other.
* A branch cannot be checked out in two worktrees at once.
* The shared files hold nothing specific to one project.
