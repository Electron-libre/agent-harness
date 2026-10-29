# Judge the text blocks between tool calls

# Goal
Rule `user-language` is mostly breached in the text blocks the agent writes between tool calls, which the Stop hook
`language-hint` never judges. Send the agent back on those blocks too, so the share of sessions breaching the rule
falls, and remove the two skews that make the hook and the measure misjudge a text.

# Findings
* Measure of 2026-10-29 (`issues/measure_the_effect_of_the_language_hint.md`, `# Tracking`): before the hook, 18 of
  the 20 breaches of `user-language` were intermediate blocks (assistant text followed by more assistant text before
  the next typed message), 2 were final replies; with the hook, 4 breaches, all intermediate. The hook ran on 157
  stops of 51 sessions and no transcript holds its hint (how a blocking Stop hook shows in `stop_hook_summary` was
  not checked).
* The English stop-word list of `crates/cardinal/src/tools/language.rs` holds `an` and `as`, also French words
  (`un an`, `tu as`).
* `Language::named` matches only the exact strings `French` and `English`: `user_language = "french"` turns the rule
  and the hook off silently.

# Approach
1. Check which hook event sees the text block of an assistant message before its tool call (`PreToolUse` or
   `PostToolUse` with `transcript_path`), and how each feeds a message back to the agent without failing the call.
2. Candidates, can be combined:
   1. A `PostToolUse` hook that judges the assistant text written since the last tool result or typed message, with
      the test of BRE-10, and tells the agent once per turn to write in the `user_language`.
   2. A `PreToolUse` hook doing the same, through added context rather than a denied call if the event allows it
      (a denied call costs a retry of the tool).
   3. Extend `language-hint` to judge every text block of the turn at Stop: the hint comes after the fact, the user
      already read the blocks.
3. Drop `an` and `as` from the English list; match the language name without case, or warn on an unknown name.
4. Measure again with `cardinal run rule-breaches` once sessions ran with the change, final and intermediate blocks
   counted apart.

# Specs
* `specs/language-hint.md`: its non-goal on intermediate blocks changes; new requirements for the chosen event.
* `specs/rule-breaches.md`: BRE-10 (stop-word lists, language name).

# Resources
* `specs/language-hint.md`, `specs/rule-breaches.md`, ADR 0005
* `crates/cardinal/src/tools/language_hint.rs`, `crates/cardinal/src/tools/language.rs`
* `entries/.claude/settings.json` (where a new hook is declared)
* `issues/measure_the_effect_of_the_language_hint.md` (the measure and its classification of the breaches)

# Hints
* A breach the hook caught and the agent then fixed still shows as a breach of BRE-10: count the corrected blocks
  apart, or the measure hides the effect.
* The transcript writes each content block of a message on its own line sharing one `message.id`: the line of the
  tool call holds no text.
* Changing the stop-word lists changes what BRE-10 counts: re-run the baseline with the new lists on the transcripts
  still kept (pruning removes the old ones) before comparing with the table of 2026-10-29.
* One hint per turn at most: a hook firing on every tool call of a long turn floods the context.
