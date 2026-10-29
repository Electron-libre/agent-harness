# Measure the effect of the hint between tool calls

# Goal
The `PostToolUse` mode of `language-hint` (commit `0a12992`) adds a context once per turn when the text written
before a tool call is not in the `user_language`. Measure whether the share of sessions breaching rule
`user-language` fell, and whether the hint reaches the agent, then decide whether to keep the mode, change it or drop
it.

# Findings
* Baseline of 2026-10-29 (`issues/done/judge_the_text_blocks_between_tool_calls.md`, `### Baseline`): before the Stop hook,
  9 of 89 sessions breached (21 breaches: 2 final, 19 intermediate); with the Stop hook, 4 of 29 (4 intermediate),
  revisions split by `git merge-base --is-ancestor ad56d2a <revision>`.
* `rule-breaches` marks each `user-language` breach `hinted`, `final` or `intermediate` (BRE-13). The hint shows in a
  transcript as an `attachment` line of type `hook_additional_context`, or as meta Stop hook feedback, holding
  `the user_language of .claude/cardinal.toml`.
* Not verified when the mode shipped: that Claude Code shows the added context to the model in a real session.

# Approach
1. Wait until enough sessions ran on a revision holding `0a12992` (about as many as the baseline's group with the Stop
   hook, 29 sessions applied).
2. Run `cardinal run rule-breaches` (summary and detail), grouped as the baseline (before the Stop hook, Stop hook
   only, with `0a12992`), counting `hinted`, `final` and `intermediate` apart, with the model and Claude Code version
   of each group.
3. Check in the transcripts that each hint shows as `hook_additional_context`, and whether the blocks after a hint in
   the same turn breach less than those before it.
4. Decide: keep the mode, change it (for example a hint per block rather than per turn), or drop it; file a follow-up
   when a change is needed.

# Resources
* `issues/done/judge_the_text_blocks_between_tool_calls.md` (the baseline and the decisions)
* `issues/done/measure_the_effect_of_the_language_hint.md` (the method of the previous measure)
* `specs/language-hint.md`, `specs/rule-breaches.md` (BRE-10, BRE-13), `skill:evaluate-harness`

# Hints
* Pruning removes old transcripts: re-run the baseline groups on the transcripts kept at the time of the measure
  instead of comparing with the table of 2026-10-29 alone.
* A `hinted` breach is the block the hook caught: count it apart, the effect shows in the blocks after it.
