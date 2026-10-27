# Align the end of the Tracking section in QUA-6 with the code

# Goal
QUA-6 of `specs/delivered-quality.md` says the `# Tracking` section ends "as in QUA-3", that is at the next line
starting with `#`, which would stop at `## Plan` and count no `Code review` line under `## Deliverable n`. The code
(`reviews` in `crates/harness/src/tools/delivered_quality.rs`) ends it at the next line starting with `# `. A test
written from the spec would fail against the code: make the wording say what the code does.

# Findings
* Found by the `high` review of `issues/check_the_red_run_of_traced_tests.md`; QUA-11 already says "ending at the next
  line starting with `# `".

# Approach
1. Amend QUA-6 to end the section at the next line starting with `# ` outside a fenced code block, as QUA-11.
2. Check that a test of QUA-6 has a `Code review` line under a `##` sub-heading (the fixture `ALPHA` has one).

# Specs
* `specs/delivered-quality.md`: amends QUA-6.

# Resources
* `specs/delivered-quality.md` (QUA-3, QUA-6, QUA-11), `crates/harness/src/tools/delivered_quality.rs` (`section`,
  `reviews`), `crates/harness/tests/delivered_quality.rs`
