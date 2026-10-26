# Shrink the review gate to the native hook

# Goal
The review gate costs more than it protects. `require_code_review.rs` parses shell text to find the repository a
commit lands in and to spot the waiver being written, and every review finds a new bypass that becomes a new
requirement. ADR 0004 already made the native `pre-commit` hook the gate the agent cannot route around by naming the
repository differently. Record that the agent is cooperative but fallible, not adversarial, and cut the Claude hook
down to what the native hook cannot do, so the gate stops growing.

# Findings
* `files/.claude/hooks/require_code_review.rs` is 878 lines; its test file is about 55 KB. Together they hold more
  than a third of the bytes under `files/`.
* 18 commits touched the hook, 16 on 2026-10-25 and 2 on 2026-10-26; about 10 of the 30 issues in `issues/done/` are
  about it.
* `specs/review-gate.md` has 18 requirements; REV-1 to REV-8 and REV-16 read the repository from the command text
  (`-C`, `--git-dir`, `GIT_DIR`, `export`, fish `set -x`, `cd`), REV-13, REV-14 and REV-18 read the text for a write
  of the waiver or a run of `just waive-review`.
* ADR 0004, Consequences: the text rules "are no longer what the gate rests on: the native hook decides when they
  cannot read the command".
* `issues/done/make_the_review_waiver_usable.md`: one review returned 10 findings, all about the shell parser, all
  fixed in the same deliverable.

# Approach
1. Write an ADR (superseding the relevant part of 0004) stating the threat model: the gates catch the agent's
   mistakes, not deliberate bypasses; a finding that is a bypass by a cooperative agent's unlikely command is a
   non-goal, not a requirement.
2. Keep in the Claude hook only `record` and, in `check`, the refusal of `--no-verify`, `-n` and `core.hooksPath`
   (REV-12); the native hook (`native`, REV-9 to REV-11, REV-15) judges the repository.
3. Replace the text detection of the waiver and of its recipe by a `permissions.deny` rule in the seeded
   `.claude/settings.json` (for example `Bash(just waive-review*)`), plus the existing instruction in the skill.
4. Withdraw the requirements that no longer hold (`withdrawn:`, numbers kept) and delete their tests and code.
5. Measure before and after: lines of the hook and its tests, latency of `check` on a plain `git status`.

# Specs
* `specs/review-gate.md`: withdraws REV-1 to REV-8, REV-13, REV-14, REV-16 and REV-18; may add a requirement for the
  deny rule of the seeded settings.

# Resources
* `docs/adr/0004-review-gate-moves-from-the-command-to-the-repository.md`
* `specs/review-gate.md`, `files/.claude/hooks/require_code_review.rs`, `files/.claude/hooks/test_require_code_review.rs`
* `seed/.claude/settings.json`, `files/.claude/skills/issue-workflow/SKILL.md` (step 4.5)
* `crates/harness/src/gate.rs` (installs the native hook)

# Hints
* A `permissions.deny` rule in a project's settings is applied by Claude Code, not by a script: check the exact
  pattern syntax in the Claude Code docs before relying on it.
* `harness sync` does not overwrite a consumer's `settings.json` (it is seeded): say how existing consumers get the
  deny rule.
* The hook refuses Bash commands that write the waiver file or run its recipe while it still exists: edit with the
  file tools.
