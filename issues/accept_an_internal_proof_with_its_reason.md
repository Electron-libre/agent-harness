# Accept an internal proof with its reason

# Goal
A requirement must stay writable and provable when a test through the binary is impossible, slow or fragile. Today
`specs/README.md` says a test under `src` "does not count as the proof" and `check-specs` warns of each such
requirement (TRC-1) with no way to say the choice is deliberate, so the warning pushes the agent toward long and
fragile end-to-end tests where a test at the boundary of the module was enough. A boundary test through the binary
must be an improvement made when it is cheap and stable, not a rule.

# Findings
* Raised by the user at the end of `prove_the_run_and_gate_specs_through_the_binary` (2026-10-27): the remaining
  warnings (SYN-18 to SYN-24, REV-19, BIN-9) read as work to drive to zero.
* The internal gate tests call `sync()` and `check_with()`, the public functions of the library: a boundary of the
  module, not its internals.

# Approach
1. A citation may carry its reason, `// spec: BIN-4 (internal: <reason>)`: `check-specs` reads the ids as today and
   leaves out of the TRC-1 warning a requirement whose internal citations all give a reason; an empty reason is a
   problem.
2. Reword `specs/README.md`: a test through the binary is preferred when it is cheap and stable; otherwise an internal
   test is the proof, with its reason on the citation.
3. Say the same where the agent decides the test (`skill:write-spec`, the "test first" step of
   `skill:issue-workflow`), in `files/`, then `just sync`.

# Specs
* `specs/spec-tracing.md` (TRC): a new requirement for the reasoned internal citation; TRC-1 amended.

# Resources
* `specs/README.md`, the TRC spec, `crates/harness/src/tools/check_specs.rs` (`boundary_warnings`, the citation
  parser), `crates/harness/tests/check_specs.rs`, `files/.claude/skills/write-spec/SKILL.md`,
  `files/.claude/skills/issue-workflow/SKILL.md`, `harness run delivered-quality` (reads the citations too).

# Hints
* The citation parser is shared with `delivered-quality`: the reason must not change the ids it reads.
