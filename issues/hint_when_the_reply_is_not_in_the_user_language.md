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
