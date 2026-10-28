# Hint when the reply is not in the user language

# Goal
The `user_language` rule is always-on text and the agent still breaks it. Add a `Stop` hook that reads the last
assistant message, applies the same language test as `rule-breaches`, and sends the agent back once with a short
hint when the reply is in the other language.

# Findings
* `harness run rule-breaches --summary` on 2026-10-28: rule `user-language` breached in 12 of the 142 sessions
  where it applied, 21 breaches, over 7 revisions, the latest on 2026-10-28.
* BRE-10 (`specs/rule-breaches.md`) already defines the test: a text block of at least 20 words once code is
  removed, holding more stop words of the other language; the lists are in
  `crates/harness/src/tools/rule_breaches.rs` (`stop_words`).
* `clippy-gate` is an existing `Stop` hook and lets go when `stop_hook_active` is set, which avoids a loop.

# Approach
1. Write the spec of the hook (a new capability file or requirements in an existing one): when it fires, what it
   reads (the transcript path of the `Stop` event), the hint it gives, when it lets go.
2. Share the language test between `rule-breaches` and the hook instead of copying the lists.
3. Add a `harness run` tool, declare it in `entries/.claude/settings.json` under `Stop`, then measure with
   `skill:evaluate-harness` whether `breached_share` of `user-language` drops.

# Specs
* `specs/rule-breaches.md` (BRE-10, shared test); the hook's own spec, written at `issue-workflow` step 1.

# Resources
* `specs/rule-breaches.md`, `crates/harness/src/tools/rule_breaches.rs`
* `crates/harness/src/tools/clippy_gate.rs`, `specs/clippy-gate.md` (an existing `Stop` hook)
* `entries/.claude/settings.json`

# Hints
* After a `harness sync` that adds a hook, run `just check-harness` and the `cargo install` it prints.
* A reply made mostly of code, paths or quoted output must not trigger the hint (the 20-word floor of BRE-10).

# Tracking

Plan: validated

## Understanding
A `Stop` hook `language-hint` judges the last reply with the BRE-10 test (moved to a module shared with
`rule-breaches`) and exits 2 once with a one-line hint. It reads `last_assistant_message` of the `Stop` event (Claude
Code 2.1.312 sends it), the transcript only as a fallback. The measure needs sessions run with the hook: a follow-up.

## Plan
1. [x] Shared language test: move `stop_words`, `prose_words` and the 20-word floor of `rule_breaches.rs` into
   `tools/language.rs`, one function judging a text; verified by the `rule_breaches` tests unchanged (~120 lines).
2. [ ] Hook `language-hint`: the tool, its declaration under `Stop` in `entries/.claude/settings.json`, `just sync`,
   docs (architecture table); verified by `tests/language_hint.rs` through the binary; makes true LNG-1 to LNG-6,
   spec `accepted` (~350 lines).
3. [ ] Follow-up issue to measure `breached_share` of `user-language` with `skill:evaluate-harness` once sessions ran
   with the hook (~40 lines, `issues/` only).

## Decisions
- Mode: autonomous
- Base commit: fddf3f4e14becb841336fd2e6b0e11c4976e54a9
- The measure of `breached_share` goes to a follow-up issue (deliverable 3): no session ran with the hook yet.

## Deliverable 1
- done: the stop words, the prose filter and the 20-word floor moved to `tools/language.rs` (`Language::named`,
  `breached_by`); `rule-breaches` calls it.
- | check | result |
  |---|---|
  | tests | `cargo test` green, BRE-10 test unchanged |
  | lint | clippy -D warnings clean |
  | specs | `just check-specs` ok |
  | docs | no doc names the moved code (grep of `docs/`, `README.md`, `CLAUDE.md`) |
  | code review | low, no findings |
- red: none (pure refactor under the existing BRE-10 test)
- Code review: no findings
- not verified: none
- decisions needed: none
