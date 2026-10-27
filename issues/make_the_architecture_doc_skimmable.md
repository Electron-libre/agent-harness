# Make the architecture document skimmable

# Goal
`docs/architecture.md` states the design and every edge case in long paragraphs, so neither the user nor an agent
reads it whole. Restructure it so the design reads in a few minutes and the edge cases live where they are enforced
(specs, ADRs, doc comments).

# Findings
* `docs/architecture.md`: 248 lines, about 21.7 kB, mostly paragraphs of several hundred words (the review gate, the
  transcript readers).
* Many of its edge cases are already requirements of `specs/review-gate.md`, `specs/rule-breaches.md`,
  `specs/delivered-quality.md`; `specs/README.md` says the spec says what, the ADR why, `docs/` how the code is laid
  out.

# Approach
1. Measure: list each paragraph's statements and mark those already in a spec, an ADR or a doc comment.
2. Keep in the document an overview per component (role, files, flow, one diagram), with links to the spec and ADRs;
   move or drop the duplicated edge cases.
3. Check with `skill:update-docs` that nothing that only lived there is lost.

# Resources
* `docs/architecture.md`, `specs/`, `docs/adr/`, `files/.claude/skills/update-docs/SKILL.md`

# Hints
* Do it after `ship_the_harness_scripts_as_prebuilt_binaries` if that issue lands first: it rewrites several
  sections.
