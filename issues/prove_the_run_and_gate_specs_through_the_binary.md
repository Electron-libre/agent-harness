# Prove the run and gate specs through the binary

# Goal
Move the proofs of `harness run`, the launcher, the install gate and the transcript retention from internal unit
tests to tests that run the `harness` binary on temporary directories, so these specs and their tests form an
acceptance suite that survives a rewrite of the code.

# Findings
* Split from `prove_the_specs_with_black_box_tests` by size (2026-10-27).
* Internal citations: `crates/harness/src/tests/install_gate.rs` 25 (24 `SYN`, 1 `REV`),
  `crates/harness/src/tests/run.rs` 13 (`BIN`), `crates/harness/src/tests/retention.rs` 6 (`SYN`),
  `crates/harness/src/tests/launcher.rs` 4 (`BIN`).

# Approach
1. Per file of tests, write the boundary test that proves each requirement through the command line, files, exit
   code and output; move the `// spec:` citation to it.
2. Keep an internal test that adds detail as a unit test without a citation; drop the ones the boundary test covers.

# Specs
* `specs/tool-binaries.md`, `specs/sync.md`, `specs/review-gate.md`: content unchanged, only their proofs move.

# Resources
* `crates/harness/src/tests/`, `crates/harness/tests/` (`tools_cli.rs`, `exit_codes_cli.rs`, `retention_cli.rs`)

# Hints
* After `prove_the_sync_specs_through_the_binary`: same method, its helpers for running the binary can be reused.
* The launcher fetches revisions: a boundary test must stay offline (local source or a local clone), per `CLAUDE.md`.
