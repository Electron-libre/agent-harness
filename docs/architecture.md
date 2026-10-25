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

## Hooks and their declaration

The hooks are managed (`files/.claude/hooks/`) but their declaration lives in `.claude/settings.json`, which is
seeded once, so a new hook reaches a project that already adopted the harness only when the declaration is added
to its file by hand. `harness check` stays lock-only and does not tell when one is missing. The extra hooks to
declare, under `hooks` in `.claude/settings.json`:

```json
"PreToolUse": [
  {
    "matcher": "Bash",
    "hooks": [
      { "type": "command", "command": "rust-script \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/block_ai_attribution.rs" }
    ]
  }
]
```

`block_ai_attribution.rs` refuses a `git commit` or `gh pr create|edit` whose message (inline, heredoc, `-F` or
`--body-file`) credits the agent: a co-author trailer, a "Generated with" footer or a credit line ending with the
vendor's no-reply address. It reads only the literal command and message files: a message built by `$(cat file)`
or a shell variable is out of reach. Its patterns are narrower than those of `commit_check.rs` on purpose, so a
message that discusses the rule passes.

## The instruction audit

`audit-instructions` is the one skill that reads outside the repository: the user's session transcripts, memory
and instruction files under `~/.claude/projects`, through `audit_sources.rs`. It only proposes skills and hooks;
the report holds private content, so it stays in the conversation (or a file outside the repository) and each
validated proposal becomes an issue in generic wording. It is a skill rather than a `harness` subcommand
because telling a repeated procedure from a rule needs judgment, and a skill keeps the shared files free of
project specifics.

## Sources

A directory (relative to the project) or a git URL; a git source is cloned in `$XDG_CACHE_HOME/harness` and
checked out at `rev` (default: the remote's default branch). `rev` pins a commit, not a moving name:
see [ADR 0002](adr/0002-rev-pins-the-resolved-commit.md). `harness update [--rev <name>]` resolves a name
(default: the default branch head) to a commit, previews the sync against it (`--dry-run` stops there), and only
without conflict (or with `--force`) syncs and writes the commit to `rev`; a conflict, an unknown revision or an unreadable source leave the
project as it was. A local directory has no revision and is refused.
A pinned commit that a force-push or a deleted branch left without any ref is fetched by its hash (`git fetch
origin <hash>`); when the source no longer serves it, `sync` and `update` fail naming the commit and `harness
update`. `check` only compares the project with its lock and never reads the source. The CI step installs the tool
with `cargo install --git --rev <hash>`, which depends on the same host behaviour: it can fail the same way, and the
fix is the same `harness update`.
