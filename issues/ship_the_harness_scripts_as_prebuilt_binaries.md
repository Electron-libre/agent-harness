# Ship the harness scripts as prebuilt binaries

# Goal
Every hook, check and transcript reader runs through `rust-script`, so a consumer project needs a Rust toolchain,
`rust-script` and a first compile of each script, even when the project is not written in Rust. Ship them as prebuilt
binaries downloaded by `harness`, so the only prerequisites are the `harness` binary and `just`.

# Findings
* `files/harness.just` and the hook declarations of `seed/.claude/settings.json` call `rust-script` on about fifteen
  scripts (hooks, `check_specs.rs`, `check_issue.rs`, `commit_check.rs`, `close_issue.rs`, `next_issue.rs`,
  `issue_worktree.rs`, the transcript readers).
* The transcript readers carry the SHA-256 of `transcripts.rs` because `rust-script` would otherwise keep running a
  stale build (ADR 0006): a cost of the script model.
* The CI already installs `harness` with `cargo install --git --rev <hash>`.

# Approach
1. Measure: the cold and warm start time of a hook under `rust-script` against a compiled binary, and what a
   non-Rust consumer must install today.
2. Fold the scripts into subcommands of `harness` (one binary, versioned by the pinned `rev`), or into a second
   binary; release builds per platform attached to a tag, fetched by `harness sync` into a cache.
3. Keep `rust-script` as a fallback for the repository itself, where the scripts are developed.

# Resources
* `files/harness.just`, `seed/.claude/settings.json`, `files/.claude/hooks/`, `files/.claude/transcripts/`
* `docs/adr/0001-shared-harness-synced-by-a-rust-tool.md`, `docs/adr/0006-transcript-readers-are-managed-scripts.md`

# Hints
* Depends on `propagate_new_hook_declarations_on_sync`: the hook commands change in every project's settings.
* The pinned `rev` must select the binary too, or a project runs hooks of another revision than its files.
