# Read the fences of the issue sections as the specs say

# Goal
`section` in `crates/harness/src/tools/delivered_quality.rs` cuts the sections of a done issue file (`## Plan`,
`# Tracking`) for QUA-3, QUA-6 and QUA-11 of `specs/delivered-quality.md`. Its fence handling and the wording of the
three requirements disagree, so a test written from the spec can fail against the code, and some issue files are
read wrong. Expected: one fence rule, worded once and shared by QUA-3, QUA-6 and QUA-11, that says what the code
does, and a heading search that skips fenced lines.

# Findings
* Found by the `high` review of `issues/done/align_the_tracking_section_end_of_qua_6.md`.
* `section` treats a line as a fence only when it starts at column 0 with three backticks: a `~~~` fence or an
  indented fence is not a fence. QUA-3 and QUA-6 say "outside a fenced code block": a `~~~` fence holding a
  column-0 `# build` ends `# Tracking` there, and the `Code review` lines after it are dropped.
* QUA-11 says the section ends "at the next line starting with `# `", with no fence clause, though its code goes
  through the same fence-aware `section`.
* `section` finds its heading with `skip_while` without tracking fences: a `# Tracking` quoted in a fence earlier in
  the file (a Goal quoting the template) starts the section there, and the closing fence then flips `fenced`, so
  every line up to the next fence is kept.

# Approach
1. Define the fence once in the spec (which lines open and close one), and refer QUA-3, QUA-6 and QUA-11 to it.
2. Either widen `section` to that definition (`~~~`, indented fences) or narrow the wording to what it reads today.
3. Track fences in the heading search too, with a test whose Goal quotes `# Tracking` in a fence.

# Specs
* `specs/delivered-quality.md`: amends QUA-3, QUA-6, QUA-11.

# Resources
* `specs/delivered-quality.md`, `crates/harness/src/tools/delivered_quality.rs` (`section`, `deliverables`,
  `reviews`, the red-run reader), `crates/harness/tests/delivered_quality.rs`
