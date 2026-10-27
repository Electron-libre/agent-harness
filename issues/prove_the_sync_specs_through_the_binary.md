# Prove the sync specs through the binary

# Goal
Move the proofs of `specs/sync.md` for `init`, `sync`, `update`, `check` and the managed settings entries from
internal unit tests to tests that run the `harness` binary on temporary directories, so the spec and its tests
form an acceptance suite that survives a rewrite of the code.

# Findings
* Split from `prove_the_specs_with_black_box_tests` by size (2026-10-27).
* `crates/harness/src/tests.rs` holds 32 `// spec: SYN-n` citations and `crates/harness/src/tests/settings_entries.rs`
  20, all calling internal functions (`use super::*`).
* `crates/harness/tests/` already runs the built binary (`CARGO_BIN_EXE_harness`) in several files.

# Approach
1. Per group of requirements (init and sync, update, check, settings entries), write the boundary test that proves
   each one through the command line, files, exit code and output; move the `// spec:` citation to it.
2. Keep an internal test that adds detail as a unit test without a citation; drop the ones the boundary test covers.

# Specs
* `specs/sync.md`: content unchanged, only its proofs move.

# Resources
* `crates/harness/src/tests.rs`, `crates/harness/src/tests/settings_entries.rs`, `crates/harness/tests/`
* `files/specs/README.md` (the tracing rule)

# Hints
* After `prove_the_specs_with_black_box_tests`: it sets the rule and reports the internal-only proofs this issue
  removes.
* Black-box tests through the binary are slower; keep them on temp dirs with no network, per `CLAUDE.md`.
