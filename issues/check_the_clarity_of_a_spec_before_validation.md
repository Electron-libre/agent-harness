# Check the clarity of a spec before its validation

# Goal
A spec reaches the validation stop of `issue-workflow` (step 2) with ambiguities the model guessed instead of
flagging. Make the ambiguities visible before the user validates: a marker for an open question the model cannot
settle, and mechanical checks of the wording that `check_specs.rs` can run on a `draft`.

# Findings
* `write-spec` asks for "must" / "must not" statements, but `check_specs.rs` checks the format and the traceability,
  not the wording of a statement.
* Open questions only surface in the question table of `issue-workflow` step 2, written from the model's judgment;
  nothing in the spec file marks a requirement as unsettled.
* GitHub spec-kit makes the model write `[NEEDS CLARIFICATION: ...]` rather than guess, and requires none to remain
  before planning; it also runs a checklist of completeness on the spec.

# Approach
1. A marker `[NEEDS CLARIFICATION: <question>]` allowed in a `draft` spec; `check_specs.rs` refuses it in an
   `accepted` spec, and `issue-workflow` step 2 turns each marker into a row of its question table.
2. Wording checks on each requirement (warnings on a `draft`): no "should", "may", "etc.", "appropriate", "fast";
   exactly one "must" or "must not".
3. A short checklist in `write-spec` (each requirement observable from outside, each boundary has a scenario,
   non-goals cover what a reader would assume).

# Specs
* The spec format is defined by `specs/README.md`; the checker's own behaviour may deserve a spec (none exists yet).

# Resources
* `files/.claude/specs/check_specs.rs`, `files/.claude/skills/write-spec/SKILL.md`, `files/specs/README.md`
* `files/.claude/skills/issue-workflow/SKILL.md` (step 2)
* https://github.com/github/spec-kit/blob/main/spec-driven.md

# Hints
* Keep the wording checks as warnings: a word list will have false positives in quoted text and code spans.

# Tracking

Plan: validated

## Understanding
Make the open questions and vague wording of a spec visible before step 2: `check_specs.rs` refuses a
`[NEEDS CLARIFICATION: ...]` marker in an `accepted` spec, warns on it and on vague wording in a `draft`; the skills
tell the model to write the marker instead of guessing. Out of scope: a spec of the checker's existing checks.

## Plan
1. [x] Clarification marker: `check_specs.rs` fails on a marker in an `accepted` spec, warns in a `draft`, ignores code spans; verified by new cases of `test_check_specs.rs`; makes true CLR-1, CLR-2, CLR-5 (markers).
2. [ ] Wording warnings: forbidden words and the single `must` on `draft` requirements, outside code spans and quotes, none on `accepted`; verified by new cases of `test_check_specs.rs`; makes true CLR-3, CLR-4, CLR-5, CLR-6, CLR-7; sets `specs/spec-clarity.md` `accepted`.
3. [ ] Skills and format doc: `write-spec` (marker rule, completeness checklist), `issue-workflow` step 2 (one question row per marker), `specs/README.md` (marker and warnings); verified by `just check-harness` and a reread.

## Decisions
- Mode: autonomous
- Base commit: a924123e3edcafa367cc412a29d997b5916e0a9b
- Spec scope: only the new clarity checks; the existing format and traceability checks stay out (non-goal).
- Wording warnings in `write-spec`: fix each one or justify it in the report; marker warnings go to the question table.

## Deliverable 1
- done: `check_specs.rs` fails on a `[NEEDS CLARIFICATION: ...]` marker of an `accepted` spec (file:line) and warns with the question in a `draft`; inline code spans and fenced code blocks are ignored.

| check | result |
|---|---|
| tests (`just check`, `just test-harness`) | pass |
| specs | pass (`spec-clarity` draft) |
| docs | header of `check_specs.rs`; `specs/README.md` in deliverable 3 |

- red: `FAIL - a clarification marker in an accepted spec is flagged with its line`, `FAIL - a clarification marker in a draft spec is warned of with its question`
- Code review: 1 finding
  - a marker in a fenced code block was flagged: fixed (fenced blocks skipped, test added, CLR-5 widened while `draft`).
- not verified: none
- decisions needed: none
