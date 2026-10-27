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
