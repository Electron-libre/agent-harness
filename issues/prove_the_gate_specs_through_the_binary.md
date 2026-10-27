# Prove the gate specs through the binary

# Goal
Move the proofs of the native review gate `sync` installs and of the settings warnings of `check`, `sync` and
`update` from internal unit tests to tests that run the `harness` binary, and the `pre-commit` it installs, on
temporary directories, so these specs and their tests form an acceptance suite that survives a rewrite of the code.

# Findings
* Split from `prove_the_run_and_gate_specs_through_the_binary` at its planning, by size (2026-10-27).
* Internal citations: `crates/harness/src/tests/install_gate.rs` 25 (24 `SYN`, 1 `REV`), plus BIN-9 and BIN-14
  cited with SYN-18.

# Approach
1. Gate install: run the `pre-commit` that `sync` writes with `/bin/sh` and fake `harness` or `rust-script` alone
   on the `PATH`; the move of a user's hook and its chaining (SYN-18, SYN-19, BIN-9).
2. Gate upkeep: reinstall, dry run, linked worktree, `core.hooksPath`, no repository, `check` of the gate (SYN-20 to
   SYN-23).
3. Settings warnings: deny rules, missing hooks, attribution, through `check`, `sync`, `update` and their dry runs;
   REV-19 on the shipped `entries/.claude/settings.json` (SYN-24 to SYN-27, REV-19). Then drop
   `src/tests/install_gate.rs`.

# Specs
* `specs/sync.md` (SYN-18 to SYN-27), `specs/review-gate.md` (REV-19), `specs/tool-binaries.md` (BIN-9): content
  unchanged, only their proofs move.

# Resources
* `crates/harness/src/tests/install_gate.rs`, `crates/harness/tests/common/mod.rs`,
  `crates/harness/tests/settings_entries_cli.rs`

# Hints
* After `prove_the_run_and_gate_specs_through_the_binary`: reuse the fake programs it adds to the test home.
* After `accept_an_internal_proof_with_its_reason`: move a proof to the binary only when that test stays fast and
  stable; otherwise keep the internal test and give its reason on the citation.
* Git runs isolated from the caller (`Home::isolated`): a test run from a git hook must not touch the real repository.
