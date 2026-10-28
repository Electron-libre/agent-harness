---
id: CLG
status: accepted
adrs: [0005]
---

# Clippy gate

## Intent

The Stop hook `cardinal run clippy-gate` keeps the Rust code of a project warning-free: it blocks the end of a turn while
`cargo clippy` reports a problem. Everything it prints is read by the agent and billed as tokens, so it runs clippy
only when there is something new to check and reports the problems in few lines.

## Requirements

- **CLG-1** (deterministic) The gate must exit 0 without running clippy when the event sets `stop_hook_active`, when the project has no `Cargo.toml`, and when `git status` shows no changed or untracked `.rs` file outside the `.claude` directories (the scripts of the harness are not crate code).
- **CLG-2** (deterministic) When clippy fails, the gate must exit 2 and write to stderr one line per diagnostic, each naming the file, line and column of the problem with its message, plus the errors that have no location (a build script, a missing crate), and nothing of the build progress (`Checking`, `Compiling`, `could not compile`).
- **CLG-3** (deterministic) The gate must report at most 40 diagnostics; when there are more, it must end with one line giving how many were left out.
- **CLG-4** (deterministic) After a passing run, the gate must record a stamp of the Rust state (the `HEAD` commit and the path and content of every changed or untracked `.rs` file) in the git directory of the worktree, and must exit 0 without running clippy while that state is unchanged.
- **CLG-5** (deterministic) A failing run must not record a stamp, and any change of a `.rs` file, of the `HEAD` commit or of the set of untracked `.rs` files since the stamp must run clippy again.
- **CLG-6** (deterministic) The gate must check the directory that sits, in the work tree holding the `cwd` of the event, where `CLAUDE_PROJECT_DIR` sits in its own work tree, and `CLAUDE_PROJECT_DIR` itself when the event has no `cwd` or one of the two is outside a git work tree.

## Scenarios

- (CLG-4) Given a passing run, when the agent ends another turn having edited only Markdown files, then clippy does not run.
- (CLG-5) Given a failing run, when the agent ends the next turn without touching the Rust code, then clippy runs again and reports the same problems.
- (CLG-5) Given a passing run, when the agent commits the Rust changes, then the next end of turn with a dirty `.rs` file runs clippy (the `HEAD` changed).

- (CLG-6) Given a crate in the subdirectory `backend/` of a repository and a session opened in it that entered an issue worktree, when the agent ends a turn, then the gate checks `backend/` of the issue worktree.
- (CLG-6) Given a session opened in the main worktree that entered an issue worktree, when the agent ends a turn with a clippy warning in the issue worktree, then the gate reports it, whatever the state of the main worktree.

## Non-goals

- Detecting a change that only touches `Cargo.toml`, `clippy.toml` or the toolchain: the next `.rs` edit or `just check` catches it.
- The output of the `just` recipes (`check`): they are recipes of the harness, not a requirement of this hook.
