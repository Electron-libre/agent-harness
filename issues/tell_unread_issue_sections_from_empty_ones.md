# Tell unread issue sections from empty ones

# Goal
`harness run delivered-quality` reads the sections of a done issue file with the QUA-12 fence rule, but two places
still give a wrong number instead of an honest one. Expected: the start of an issue's commits ignores a `# Tracking`
quoted in a fence, and a section that cannot be found gives empty columns, not zeros.

# Findings
* Found by the `high` review of `issues/done/read_the_fences_of_the_issue_sections_as_the_specs_say.md`.
* QUA-2 finds the oldest commit of an issue with `git log -G'^# Tracking$'` in
  `crates/harness/src/tools/delivered_quality.rs`, which does not know fences: an issue whose Goal quotes the
  template in a fence starts its commits at its creation, so `commits`, `fixes_in_issue` and `revision` count work
  done before the plan.
* `section` returns `None` when its heading is not found, but `deliverables`, `reviews` and `red_ids` turn it into an
  empty section: a fence left open before `# Tracking` (it runs to the end of the file under QUA-12) prints
  `deliverables 0`, `reviews 0`, `review_findings 0` and counts every traced id in `traced_without_red`, which reads
  as real zeros.
* `rule_breaches.rs` (`prose_words`) still reads fences its own way (backticks only, toggled), unlike the shared
  `fenced` of `crates/harness/src/tools/mod.rs`.

# Approach
1. QUA-2: list the commits touching the issue file as today, then read each version of the file (`git show`) with
   the shared fence reader to find the oldest one holding `# Tracking` outside a fence.
2. QUA-3, QUA-6, QUA-11: say in the spec that a missing section leaves its columns empty, as the transcript columns
   do when they cannot be read, and print them so.
3. Check whether BRE-10 should read fences with the shared reader too, and amend it if so.

# Specs
* `specs/delivered-quality.md`: amends QUA-2, QUA-3, QUA-6, QUA-11.
* `specs/rule-breaches.md`: maybe BRE-10.

# Resources
* `specs/delivered-quality.md` (QUA-12), `crates/harness/src/tools/delivered_quality.rs` (`issue_commits`,
  `section`), `crates/harness/src/tools/mod.rs` (`fence`, `fenced`), `crates/harness/tests/delivered_quality.rs`

# Tracking

Plan: validated

## Understanding
QUA-2 starts the commits of an issue at the oldest version of its file holding `# Tracking` outside a fence; a
section whose heading stands only inside a fence (unread) prints empty columns, while a heading absent from the file
keeps its real zeros (the five done issues closed with no `# Tracking`). BRE-10 removes fences with the shared reader.

## Plan
1. [x] Fence-aware start of an issue: read each version of the issue file touched by its candidate commits (`git
   show`) with `section` until one holds `# Tracking`; verified by a fixture issue opened with `# Tracking` quoted in
   a Goal fence (`commits` 3); makes true QUA-2. ~70 lines.
2. [x] Unread sections: `section` tells a heading found only inside a fence from an absent one, and `deliverables`,
   `reviews` and `red_ids` turn the first into empty cells; verified by a fixture issue whose Goal fence stays open
   before `# Tracking`, and one with no `# Tracking` keeping 0; makes true QUA-12 (as amended). ~80 lines.
3. [ ] BRE-10 reads fences with the shared `fenced` of `tools/mod.rs`; verified by an English text block in a `~~~`
   fence of a French session giving no breach; makes true BRE-10. ~40 lines.

## Decisions
- Mode: autonomous
- Base commit: 687d50cac9dd03efb0ff1071c4caaa10ad598bde
- Unread vs absent section (question 1 a): empty only when the heading stands only inside a fence.

## Deliverable 1
- done: `issue_commits` reads each version of the issue file (`git show`) with the fence-aware `section` and starts
  at the oldest holding `# Tracking` outside a fence, instead of `git log -G`.
- red: QUA-2: `kappa.commits: expected "3", got Some("4")` (the creation commit quoting `# Tracking` in a fence counted).
- Code review: no findings (low).
- not verified: none.
- decisions needed: none.

## Deliverable 2
- done: `readable` reads a section as empty when the file does not hold its heading and as unread (`None`) when the
  heading stands only inside a fence; `deliverables`, `reviews` and `red_ids` print empty cells for an unread one.
  The rows of this repository are unchanged.
- red: QUA-12: `theta.deliverables: expected "", got Some("0")` (a Goal fence left open before `# Tracking`).
- Code review: no findings (low).
- not verified: none.
- decisions needed: none.
