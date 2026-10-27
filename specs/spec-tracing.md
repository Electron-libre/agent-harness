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
(`tests/`, the managed tests under `.claude/`) drives the boundary. A boundary proof is preferred when it is cheap and
stable, not required: an internal test is the proof when its citation gives the reason, `// spec: ABC-1 (internal:
<reason>)`. The checker reports the requirements whose internal proofs give no reason, as a warning.

## Requirements

- **TRC-1** (deterministic) For each spec holding requirements cited only by tests under a `src` directory, one of those citations at least giving no reason (TRC-6), the checker must print one warning to stderr naming the spec file, the count and the ids of those requirements, without failing.
- **TRC-2** (deterministic) The checker must not list in that warning a requirement that a test outside every `src` directory also cites.
- **TRC-3** (deterministic) The checker must not list in that warning a requirement that no test cites.
- **TRC-4** (deterministic) The checker must not list in that warning a withdrawn requirement.
- **TRC-5** (deterministic) The checker must count a test under `.claude/` as outside every `src` directory, whatever its path below.
- **TRC-6** (deterministic) The checker must not list in that warning a requirement whose every citation under a `src` directory carries a reason, the text `(internal: <reason>)` after its ids with a reason that is not blank.
- **TRC-7** (deterministic) The checker must report as a problem, naming its file and line, a citation whose `(internal: ...)` reason is blank, does not end the line with its `)`, or follows no id.
- **TRC-8** (deterministic) The checker must read from a citation with a reason the same requirement ids as from that citation without it.

## Scenarios

- (TRC-1) Given `SYN-2` cited only in `crates/harness/src/tests.rs`, when the checker runs, then stderr holds one line
  naming `specs/sync.md`, `1` and `SYN-2`, and the exit code is the one the other checks give.
- (TRC-2) Given `SYN-1` cited in `crates/harness/src/tests.rs` and in `crates/harness/tests/sync_cli.rs`, when the
  checker runs, then `SYN-1` is not in the warning.
- (TRC-1) Given a spec whose cited requirements all have a test outside `src`, when the checker runs, then it prints
  no boundary warning for that spec.
- (TRC-6) Given `SYN-2` cited in `crates/harness/src/a.rs` with `// spec: SYN-2 (internal: needs a fake clock)` and in
  `crates/harness/src/b.rs` with `// spec: SYN-2`, when the checker runs, then `SYN-2` is in the warning.
- (TRC-7) Given `// spec: SYN-2 (internal: )`, when the checker runs, then it fails naming the file and line.
- (TRC-7) Given `// spec: SYN-2 (internal: slow) SYN-3`, `// spec: SYN-2 (internal: slow` or
  `// spec: (internal: slow)`, when the checker runs, then it fails naming the file and line.
- (TRC-8) Given `// spec: SYN-2, SYN-3 (internal: the SYN-9 fixture is slow)`, when the checker runs, then it reads
  `SYN-2` and `SYN-3`, flags no unknown id, and `SYN-9` is not cited.

## Non-goals

- Checking that a test outside `src` really runs the program: the place of the test is the convention.
- Failing the check on an internal-only proof: a boundary proof is an improvement, not a rule.
- Judging whether a reason is good: the checker only requires one.
- The other format and traceability checks of `check-specs`.
