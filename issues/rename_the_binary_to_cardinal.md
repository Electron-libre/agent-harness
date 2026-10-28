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
