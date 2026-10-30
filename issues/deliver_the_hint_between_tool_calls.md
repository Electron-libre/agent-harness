# Deliver the hint between tool calls

# Goal
The `PostToolUse` mode of `language-hint` (commit `0a12992`) never reached the agent in a real session observed so
far, though the hook prints its added context when replayed. Find why and make the hint arrive, or drop the mode if
it cannot arrive cheaply: the Stop hook already keeps the final replies in the `user_language`, so the effort stays low.

# Findings
* 2026-10-30, Opus 5.5: two text blocks not in the `user_language` written before a tool call got no
  `hook_additional_context` attachment in their transcript, and the agent of the second got no added context:
  * a session of a consumer project, Claude Code 2.1.312, installed `cardinal` built after `0a12992`, the hook
    declared in its `.claude/settings.json` (session `6df30560`, transcript line 254);
  * a live probe in this repository (session `a4bd7eec`, transcript line 155, a harmless `date` call).
* Replaying the installed `cardinal run language-hint` with a `PostToolUse` event on the same transcripts (cut after
  the `tool_use` line, or whole) prints the added context, exit 0.
* Cut before the `tool_use` line, the replay prints nothing: the hook stays silent when that line is not written yet.
* Other `PostToolUse` hooks (`format-code`) show in the transcripts as `hook_additional_context`, so the channel works.

# Approach
1. Measure first: run one session with `claude --debug` (or a hook that copies its stdin and the transcript size to a
   temporary file) and look whether `language-hint` runs on `PostToolUse` and whether the `tool_use` line is in the
   transcript at that moment.
2. If the transcript lags: judge the text the event itself carries, if any, or wait for it in a bounded way.
3. If the hook does not run or its output is dropped: drop the `PostToolUse` mode and its requirements (LNG-7 to
   LNG-10), keeping the Stop hook.

# Specs
* `specs/language-hint.md`: LNG-7 to LNG-10 may be amended or removed.

# Resources
* `crates/cardinal/src/tools/language_hint.rs` (`after_call`, `text_before_call`)
* `specs/language-hint.md`, `issues/measure_the_effect_of_the_hint_between_tool_calls.md` (`# Tracking`)

# Hints
* The user finds the language hints already work rather well: keep this small.
