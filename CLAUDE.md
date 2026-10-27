# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project

The shared agent harness (skills, hooks, scripts, recipes, CI) of the user's projects, and the Rust tool
`harness` that copies it into each of them. → Design: [docs/architecture.md](docs/architecture.md)

## Index

- `files/`: the managed content, copied to the same path of every consumer. Never edit the copies in a
  consumer: fix them here.
- `seed/`: project-owned starting points, copied once.
- `entries/.claude/settings.json`: the hooks and deny rules `sync` keeps in each project's `.claude/settings.json`
  (declare a new hook here).
- `crates/harness/`: the `harness` binary (`init`, `sync`, `check`, `run` for the tools of `src/tools/`).
- `specs/`: what the tool and its shared hooks must do (one file per capability, see `specs/README.md`), traced to
  its tests by `// spec: ID`.
- `issues/`: open issues, ordered in `issues/queue.md` (`just next-issue`); `docs/adr/`: decisions.
- Parallel agents: one git worktree per issue, started by `issue-workflow` (`just start-issue`, `just finish-issue`), see
  `docs/architecture.md`.
- `AGENTS.md`: commit message format (Conventional Commits, enforced by `just check-commit`) and the output rule for
  the reports to the user (section "Reports to the user").

This repository is its own first consumer: its `.claude/`, `AGENTS.md`, `harness.just`, `script/` and
`specs/README.md` are copies of `files/`. After changing `files/`, run `just sync`; `just check-harness`
fails when a copy drifted.

Essentials: `just check` (fmt + clippy -D warnings + tests + spec traceability, needs `rust-script`) before
committing and `just test-harness` after touching `files/`; no `Co-Authored-By` or AI attribution in commits;
everything in English; the shared files hold nothing specific to one project (project values go through
`.claude/harness.toml`).

## Test conventions

No network in unit tests; filesystem tests use `tempfile::tempdir()`. A test that proves a requirement of
`specs/sync.md` carries `// spec: SYN-n` right above it.
