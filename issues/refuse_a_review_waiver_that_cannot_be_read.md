# Refuse a review waiver that cannot be read

# Goal
The native commit gate reads a waiver it cannot read as empty, stamps it and lets the commit through; at the next
commit it reads it as empty again, so that waiver is never spent and waives every later review. A waiver the hook
cannot read should waive nothing, as one it cannot stamp already does.

# Findings
* `files/.claude/hooks/require_code_review.rs`, `refusal()`: `std::fs::read_to_string(w).unwrap_or_default()` turns a
  read failure into an empty waiver; the write of the stamp succeeds when the file is writable (mode `0200`).
* Found by the end-of-issue `high` review of `tell_the_agent_to_ask_the_user_when_the_waiver_cannot_be_stamped`.

# Approach
1. Treat a waiver that exists but cannot be read like one that cannot be stamped: it waives nothing, the review judges
   the commit and a refusal tells the agent to ask the user to fix that file (path named).
2. Amend REV-10 (or add a requirement) of `specs/review-gate.md` for the unreadable case.

# Specs
* `specs/review-gate.md`: REV-10 amended or a new requirement (unreadable waiver).

# Resources
* `files/.claude/hooks/require_code_review.rs` (`refusal()`, `Refusal`)
* `files/.claude/hooks/test_require_code_review.rs` (`native_checks`, the read-only waiver case)
* `docs/adr/0005-the-gates-catch-mistakes-not-bypasses.md`

# Hints
* Root reads and writes any file mode: a test that builds the case must skip it when the file stays readable.
