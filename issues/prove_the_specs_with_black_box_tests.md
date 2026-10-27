# Prove the specs with black-box tests

# Goal
A spec describes behaviour, so it should survive a rewrite of the code in another architecture or language. Its tests
should too: today most requirements of `harness` itself are proven by unit tests that call internal functions, so a
reimplementation would have to rewrite the tests before it could be checked against the spec. Make the tests that
trace a requirement drive the program from its boundary (command line, files, exit code, output), so the spec and
its tests together are a portable acceptance suite. This issue sets the rule and its report; the proofs are moved
by the two issues split from it.

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
2. Have `check_specs.rs` report, per spec, the requirements that only have internal proofs (a warning, so the
   existing internal proofs do not fail the check until they are moved).
3. The proofs themselves move in `prove_the_sync_specs_through_the_binary` and
   `prove_the_run_and_gate_specs_through_the_binary`, split from this issue by size.

# Specs
* `specs/README.md` (the tracing rule); `specs/sync.md` unchanged in content.

# Resources
* `crates/harness/src/tests.rs`, `crates/harness/src/tests/`, `crates/harness/tests/` (boundary tests already there)
* `files/specs/README.md`, `files/.claude/specs/check_specs.rs`

# Hints
* Split by size (2026-10-27, `skill:prioritize-issues`): 105 citations of `crates/harness/src` are internal; moving
  them all is several thousand lines.
* Related to `check_the_red_run_of_traced_tests`: both make a citation prove more.

# Tracking

Plan: validated

## Understanding
Set the rule (a requirement is proven by a test that drives the program from outside `src/`) and have `check-specs`
warn, per spec, of the requirements proven only from inside `src/`. Moving the proofs is out of scope (two split issues).
Measured on 2026-10-27: 49 requirements have internal proofs only (SYN 35, BIN 13, REV 1); every other spec is proven at the boundary.

## Plan
1. [x] Boundary warning: the rule in `files/specs/README.md` (synced), the per-spec warning in
   `crates/harness/src/tools/check_specs.rs`, new spec `specs/spec-tracing.md`; verified by tests of
   `crates/harness/tests/check_specs.rs` running the binary on temp repos; makes true TRC-1, TRC-2, TRC-3 (~200 lines).

## Decisions
- Mode: autonomous
- Base commit: 306da7dd1679d752348dc0bdaddcedfddaf3fb94
- A test is internal when it sits under a `src` directory, boundary elsewhere (by place, any language).
- The requirements of the warning go in a new spec `specs/spec-tracing.md` (TRC), not in `spec-clarity.md`.
- One warning line per spec (count and ids), not one per requirement.

## Deliverable 1
- done: rule in `specs/README.md`; `check-specs` warns per spec of the requirements cited only under `src`; spec
  `spec-tracing.md` (TRC) accepted. On this repo: SYN 35, BIN 13, REV 1, as measured.
- red: TRC-1, TRC-2, TRC-3: `FAIL - requirements proven only under src are warned of in one line`, `FAIL - a
  requirement also proven outside src is not listed`, `FAIL - an uncited requirement is not listed`
- Code review: no findings
- not verified: none
- decisions needed: none
