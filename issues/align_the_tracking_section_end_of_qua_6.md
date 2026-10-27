# Align the end of the Tracking section in QUA-6 with the code

# Goal
QUA-6 of `specs/delivered-quality.md` says the `# Tracking` section ends "as in QUA-3", that is at the next line
starting with `#`, which would stop at `## Plan` and count no `Code review` line under `## Deliverable n`. The code
(`reviews` in `crates/harness/src/tools/delivered_quality.rs`) ends it at the next line starting with `# `. A test
written from the spec would fail against the code: make the wording say what the code does.

# Findings
* Found by the `high` review of `issues/done/check_the_red_run_of_traced_tests.md`; QUA-11 already says "ending at the next
  line starting with `# `".

# Approach
1. Amend QUA-6 to end the section at the next line starting with `# ` outside a fenced code block, as QUA-11.
2. Check that a test of QUA-6 has a `Code review` line under a `##` sub-heading (the fixture `ALPHA` has one).

# Specs
* `specs/delivered-quality.md`: amends QUA-6.

# Resources
* `specs/delivered-quality.md` (QUA-3, QUA-6, QUA-11), `crates/harness/src/tools/delivered_quality.rs` (`section`,
  `reviews`), `crates/harness/tests/delivered_quality.rs`

# Tracking

Plan: validated

## Understanding
Amend the wording of QUA-6 so the `# Tracking` section ends at the next line starting with `# ` (as the code and
QUA-11 do), not at any `#` line. Out of scope: any change of the code. The existing QUA-6 test already covers it.

## Plan
1. [x] Amend QUA-6: end the section at the next `# ` line outside a fence; verified by `just check` and the
   existing QUA-6 test (fixture `ALPHA` has its `Code review` lines under `## Deliverable 1`); makes true QUA-6.

## Decisions
- Mode: autonomous
- Base commit: `2abdb79c41abfc003244ca62f4284b8c9661ea24`

## Deliverable 1
- done: QUA-6 ends the `# Tracking` section at the next line starting with `# ` outside a fence, as the code and QUA-11.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass (`just check-specs`) |
| docs | no other text cites the old ending |
| code review | low, see below |

- red: none (wording only; the existing QUA-6 test already passes against the code)
- Code review: 0 findings
- not verified: nothing
- decisions needed: none
