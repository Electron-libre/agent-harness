# Rename the binary to cardinal

# Goal
Rename the crate and the binary `harness` to `cardinal` (`cardinal sync`, `cardinal run <tool>`), with the hooks,
CI, release assets and cache following, without breaking a consumer that still has the old binary installed.

# Findings
* `harness run` appears in 174 lines of 71 files; `harness sync|check|init|update` in 67 lines of 32 files.
* The managed hooks of `entries/.claude/settings.json` call `harness run review-gate`, `format-rust`, `clippy-gate`,
  `language-hint`; the git hook of `crates/harness/src/gate.rs` calls `harness help run` and `harness run`.
* `release.yml` publishes `harness-<commit>` releases with `harness-<target>` assets; `run.rs` caches binaries under
  `<cache>/harness/bin/<commit>/<target>/harness`; `install_harness.sh` installs `crates/harness`.
* `justfile` recipes run `cargo run -p harness` and `cargo install --path crates/harness`.

# Approach
1. `git mv crates/harness crates/cardinal` alone in a first deliverable (renames only).
2. Rename the package and binary, the cache directory, the release and asset names, the install script and the
   workflow names.
3. Transition for the consumers: candidates (to decide) are a `harness` alias binary installed alongside for a
   while, the hooks calling `cardinal` only after `sync` checked it is on the path (as BIN-16 does for a new hook), or
   the launcher accepting both release names.

# Specs
* `specs/tool-binaries.md` (BIN-n naming the binary, its releases and cache) and every spec citing `harness run`;
  amended at `issue-workflow` step 1.

# Resources
* `crates/harness/src/run.rs`, `launcher.rs`, `gate.rs`, `entries.rs`, `entries/.claude/settings.json`
* `.github/install_harness.sh`, `.github/workflows/release.yml`, `harness.yml`, `ci.yml`, `justfile`
* ADR 0010 (the tools ship in the harness binary)

# Hints
* After `name_the_product_cardinal`.
* A move that also edits counts twice in the size limit: move first, edit after.
* Releases already published under `harness-<commit>` stay the ones old pinned revisions download.

# Tracking

Plan: validated

## Understanding
The crate and binary become `cardinal`; its cache, releases and install script follow. A `harness` binary installed
with it (BIN-18) and a launcher that still reads the `harness-<commit>` releases (BIN-3) keep the consumers running
through the switch. Out of scope: `harness.toml`, `harness.just`, `harness.yml` and the harness-named tools (their
own issues).

## Plan
1. [x] Move the crate: `git mv crates/harness crates/cardinal` and `git mv` of both `install_harness.sh` to
   `install_cardinal.sh`, with only the paths that name them (workspace member, `justfile`, `harness.yml`, the install
   line of `launcher.rs`, docs); verified by `just check` and `just check-harness`.
   - done: crate and install script moved (77 renames, 28 lines edited), copies synced; the BIN and SYN-23 spec
     drafts ride along.
   - check | result: tests, lint, specs pass (`just check`); `just check-harness` passes; docs: paths only.
   - red: none (a move, no behaviour).
   - Code review: no findings.
   - not verified: the CI jobs that run `install_cardinal.sh` (after the push).
   - decisions needed: none.
2. [ ] Rename the package and binary: package `cardinal` with the bins `cardinal` and `harness`, cache
   `<cache>/cardinal/bin/<commit>/<target>/cardinal`, release `cardinal-<commit>` then `harness-<commit>`, a cargo
   build naming no crate, the local build of `cardinal`, the messages; verified by `run_cli.rs`, `launcher_cli.rs`;
   makes true BIN-1 to BIN-6, BIN-11, BIN-12, BIN-17, BIN-18.
3. [ ] Hook transition: the git gate runs `cardinal`, else `harness`; the review gate is found under both commands;
   `check` and `update` judge the `cardinal` hooks and print the install with `--force`; verified by `gate_cli.rs`,
   `launcher_cli.rs`, `update_cli.rs`; makes true BIN-9, BIN-15, BIN-16, SYN-23.
4. [ ] Release and CI: `release.yml` publishes `cardinal-<commit>` and the legacy `harness-<commit>`,
   `install_cardinal.sh` fetches and caches `cardinal`; verified by `just check`, the shell syntax (`sh -n`) and the
   CI run after the push.
5. [ ] Switch the commands: the managed hooks, `files/harness.just`, the skills, `AGENTS.md`, the specs, the docs and
   the `justfile` call `cardinal`; `just install` then `just sync`; follow-up issue to drop the alias; verified by
   `just check`, `just check-harness`; makes true BIN-8; sets `tool-binaries` and `sync` `accepted`.

## Decisions
- Mode: autonomous
- Base commit: 428ec4ae5ddbb574760d8ef156a45e48c21b6437
- Transition: the `harness` alias bin, the launcher reading `cardinal-<commit>` then `harness-<commit>`, `check` and
  `update` judging the `cardinal` hooks (BIN-15, BIN-16), `--force` in the install lines.
- `release.yml` also publishes each new commit as `harness-<commit>` / `harness-<target>` until the alias is dropped,
  so a launcher installed before the rename keeps fetching new revisions.
