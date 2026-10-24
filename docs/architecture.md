# Architecture

## What is shared, and how

A consumer project holds a `.claude/harness.toml` with the `[harness]` table (`source`, and `rev` for a git
source) and its own values (`[project]`, read by the scripts and the skills). `harness sync` reads the source and:

- copies every file under `files/` to the same relative path (managed: scripts, tests, hooks, skills, `harness.just`,
  `AGENTS.md`, `specs/README.md`, the CI workflow);
- copies every file under `seed/` once, when the project lacks it (project-owned: `.claude/harness.toml`
  defaults, `.claude/settings.json`, `.claude/update-docs-map.md`, `justfile`, `issues/queue.md`).

`.claude/harness.lock` holds the hash of each managed file as written. A managed file whose hash differs from the
lock was edited locally: `sync` reports a conflict instead of overwriting it, and `harness check` fails. The fix
belongs in this repository, then `sync` everywhere. There is deliberately no merge.

## Making files identical

Everything that differs between projects is a value, not code. Scripts read lists from `.claude/harness.toml`
(`commit_scopes`, `root_docs`, `skipped_dirs`) with a std-only reader; skills tell the agent which key to read
(`roadmap`, `architecture`, `test_conventions`, `confidential`, `user_language`). The change-to-documents table of
`update-docs` lives in the project's `.claude/update-docs-map.md`. Anything else that is project-specific
(quality benchmark, smoke test, the language-specific recipes of the `justfile`) stays in the project.

## Sources

A directory (relative to the project) or a git URL; a git source is cloned in `$XDG_CACHE_HOME/harness` and
checked out at `rev` (default: the remote's default branch). `rev` pins a commit, not a moving name:
see [ADR 0002](adr/0002-rev-pins-the-resolved-commit.md).
