---
id: TRC
status: accepted
adrs: []
---

# Boundary proofs of the specs

## Intent

A spec and the tests that trace it are meant to survive a rewrite of the code, so a requirement is proven by a test
that drives the program from its boundary (command line, files, exit code, output). `check-specs` tells such a test
from an internal one by where it sits: a test under a `src` directory calls the code from inside; a test elsewhere
(`tests/`, the managed tests under `.claude/`) drives the boundary. It reports the requirements that only have
internal proofs, as a warning until those proofs are moved.

## Requirements

- **TRC-1** (deterministic) For each spec holding requirements cited only by tests under a `src` directory, the checker must print one warning to stderr naming the spec file, the count and the ids of those requirements, without failing.
- **TRC-2** (deterministic) The checker must not list in that warning a requirement that a test outside every `src` directory also cites.
- **TRC-3** (deterministic) The checker must not list in that warning a requirement that no test cites.

## Scenarios

- (TRC-1) Given `SYN-2` cited only in `crates/harness/src/tests.rs`, when the checker runs, then stderr holds one line
  naming `specs/sync.md`, `1` and `SYN-2`, and the exit code is the one the other checks give.
- (TRC-2) Given `SYN-1` cited in `crates/harness/src/tests.rs` and in `crates/harness/tests/sync_cli.rs`, when the
  checker runs, then `SYN-1` is not in the warning.
- (TRC-1) Given a spec whose cited requirements all have a test outside `src`, when the checker runs, then it prints
  no boundary warning for that spec.

## Non-goals

- Checking that a test outside `src` really runs the program: the place of the test is the convention.
- Failing the check on an internal-only proof: that waits for `prove_the_sync_specs_through_the_binary` and
  `prove_the_run_and_gate_specs_through_the_binary`.
- The other format and traceability checks of `check-specs`.
