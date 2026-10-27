# Prove the specs with black-box tests

# Goal
A spec describes behaviour, so it should survive a rewrite of the code in another architecture or language. Its tests
should too: today most requirements of `harness` itself are proven by unit tests that call internal functions, so a
reimplementation would have to rewrite the tests before it could be checked against the spec. Make the tests that
trace a requirement drive the program from its boundary (command line, files, exit code, output), so the spec and
its tests together are a portable acceptance suite.

# Findings
* `specs/sync.md` (SYN) is cited by 61 tests: 60 in `crates/harness/src/tests.rs` and `src/tests/`
  (`use super::*`, internal functions), 1 in `crates/harness/tests/retention_cli.rs`, which runs the built binary
  (`CARGO_BIN_EXE_harness`).
* The tests of the managed scripts (`files/.claude/**/test_*.rs`) mostly run the script as a process
  (`rust-script`), so they are already close to black-box.
* Most requirements are already worded at the boundary ("`start-issue` must create a worktree ...").

# Approach
Measure first: classify every `// spec:` citation as boundary (runs the binary or script) or internal, per spec.
1. A rule in `specs/README.md`: a requirement is traced by a test that drives the program from its boundary; an
   internal test may add detail but does not count as the proof.
2. Move or duplicate the SYN proofs into `crates/harness/tests/` running the binary on temp directories, keeping the
   internal tests that are worth it as unit tests without a `// spec:` citation.
3. Optionally have `check_specs.rs` report, per spec, how many requirements only have internal proofs.

# Specs
* `specs/README.md` (the tracing rule); `specs/sync.md` unchanged in content.

# Resources
* `crates/harness/src/tests.rs`, `crates/harness/src/tests/`, `crates/harness/tests/retention_cli.rs`
* `files/specs/README.md`, `files/.claude/specs/check_specs.rs`

# Hints
* Black-box tests through the binary are slower; keep them on temp dirs with no network, per `CLAUDE.md`.
* Related to `check_the_red_run_of_traced_tests`: both make a citation prove more.
