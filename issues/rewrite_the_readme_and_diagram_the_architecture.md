# Rewrite the README and diagram the architecture and the processes

# Goal
The README is dense and does not tell at a glance what a reader wants to know: what the harness is, what it gives a
project, how to install and use it in a few commands, where to go next. `docs/architecture.md` states the design and
every edge case in long paragraphs, so neither the user nor an agent reads it whole. Rewrite the README as a short
entry point, make the architecture document skimmable, and show the architecture and the processes (sync, hooks and
gates, issue workflow with its worktrees, evaluation from the transcripts) as mermaid diagrams.

# Findings
* `README.md`: 67 lines, 5.3 kB; after a two-line intro, a command block whose comments run over several lines, then
  long paragraphs on the installed launcher, the transcript readers (`session-metrics`, `rule-breaches`, `delivered-quality`,
  `fixed-context`, `harness-report`) and on the ignore settings of eight tools for `.claude/worktrees/`. No section
  headings, no list of what a consumer project gets (skills, hooks, recipes, CI), no diagram.
* `docs/architecture.md`: 330 lines, 30.4 kB, 9 sections, mostly paragraphs of several hundred words (hooks and
  their declaration: about 100 lines; the evaluation from the transcripts: about 70 lines); no diagram.
* No mermaid diagram anywhere in `README.md` or `docs/`.
* Many edge cases of the architecture document are already requirements of `specs/` (review gate, rule breaches,
  delivered quality, sync) or ADRs (`docs/adr/0001` to `0011`); `specs/README.md` says the spec says what, the ADR
  why, `docs/` how the code is laid out.

# Approach
1. Measure: list what the README and each paragraph of the architecture document state, and mark what is already in
   a spec, an ADR, a skill or a doc comment.
2. README: what it is (one paragraph), what a project gets, install and the main commands (one line each), the daily
   flow (issue to commit), links to the architecture, the specs and the ADRs. Move the reference details (transcript
   readers, worktree ignore settings) to `docs/` (e.g. a usage or reference page) or to the help of the commands.
3. Architecture: an overview per component (role, files, flow, links to the spec and ADRs), duplicated edge cases moved
   or dropped.
4. Diagrams (mermaid, rendered by GitLab and GitHub): the components and what `sync` copies where; the hook and gate
   chain of a commit; the issue workflow and its worktrees (`start-issue`, deliverables, `finish-issue`); the
   evaluation pipeline (transcripts, readers, report). Pick which ones go in the README and which in `docs/`.
5. Check with `skill:update-docs` that nothing that only lived in the old text is lost.

# Resources
* `README.md`, `docs/architecture.md`, `docs/adr/`, `specs/`, `files/.claude/skills/update-docs/SKILL.md`,
  `files/.claude/skills/issue-workflow/SKILL.md`, `justfile`, `harness.just`

# Hints
* `README.md` and `docs/architecture.md` are project-owned, not copies of `files/`: no `just sync` needed for them.
* The README of a consumer is not touched by `sync`; this issue is about this repository's documentation only.
* Diagrams stay true only if small: one idea per diagram, names that exist in the code.
* `prove_the_gate_specs_through_the_binary` is still open and may move more edge cases into the specs; leave the
  gate sections short and link the spec rather than restating it.
