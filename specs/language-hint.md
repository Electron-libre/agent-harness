---
id: LNG
status: draft
adrs: [0005]
---

# Language hint

## Intent

The hook `cardinal run language-hint` sends the agent back once when its last reply is not in the `user_language` of
`.claude/cardinal.toml` (a Stop hook), and tells it once per turn when the text it wrote before a tool call is not (a
`PostToolUse` hook), a rule the agent breaks although it is always in its context, mostly in the text between tool
calls. It judges the text with the test of rule `user-language` (BRE-10, `specs/rule-breaches.md`), so the hook and
the measure agree on what a breach is.

## Requirements

- **LNG-1** (deterministic) On a Stop event, the hook must exit 0 and print nothing when the event sets `stop_hook_active`.
- **LNG-2** (deterministic) On a Stop event (any `hook_event_name` but `PostToolUse`), the hook must judge the `last_assistant_message` of the event, and, when the event has none, the text blocks of the last assistant line holding text in the transcript at its `transcript_path`, joined by a blank line, provided no message the user typed (the filter of `transcripts.rs`) comes after that line.
- **LNG-3** (deterministic) The hook must read `user_language` in the `[project]` table of the configuration (`.claude/cardinal.toml`) at the root of the work tree holding the `cwd` of the event, and in that of `CLAUDE_PROJECT_DIR` when the event has no `cwd`, it is outside a git work tree or that work tree has neither.
- **LNG-4** (deterministic) The hook must judge the text with the same function as BRE-10: at least 20 words once inline code and fenced code blocks are removed, and more stop words of the other language than of `user_language`.
- **LNG-5** (deterministic) On a Stop event, when the text breaks the test of LNG-4, the hook must exit 2 with exactly one line on stderr that names `user_language`, holds `the user_language of .claude/cardinal.toml` and asks to give the reply again in it, quoting no text of the reply.
- **LNG-6** (deterministic) The hook must exit 0 and print nothing when the text passes the test of LNG-4, holds fewer than 20 words of prose, or cannot be judged (an unreadable event, transcript or config, no `user_language`, a language with no stop words).
- **LNG-7** (deterministic) On a `PostToolUse` event, the hook must judge the text blocks of the assistant lines of the transcript at `transcript_path` that come before the line holding the `tool_use` block of the event's `tool_use_id`, back to the previous user line (a tool result or a typed message), joined by a blank line, and nothing when that `tool_use` block is not in the transcript or another `tool_use` block comes between those texts and it (a parallel call of the same message).
- **LNG-8** (deterministic) On a `PostToolUse` event, the hook must judge nothing when an `attachment` line of type `hook_additional_context` whose content holds `the user_language of .claude/cardinal.toml` comes after the last message the user typed (the filter of `transcripts.rs`) in the transcript: one hint per turn.
- **LNG-9** (deterministic) On a `PostToolUse` event, when the text breaks the test of LNG-4, the hook must exit 0 and print on stdout one JSON object whose `hookSpecificOutput` holds `hookEventName` `PostToolUse` and an `additionalContext` of one line that names `user_language`, holds `the user_language of .claude/cardinal.toml` and asks to write the next texts to the user in it, quoting no text of the agent.
- **LNG-10** (deterministic) On a `PostToolUse` event, the hook must exit 0 and print nothing when the text passes the test of LNG-4, holds fewer than 20 words of prose, or cannot be judged (the cases of LNG-6).

## Scenarios

- (LNG-5) Given `user_language = "French"` and a last reply of 30 English words, when the agent stops, then the hook
  exits 2 with the hint; when the agent stops again (`stop_hook_active`), it exits 0 (LNG-1).
- (LNG-6) Given `user_language = "French"` and a reply made of a fenced Rust block and one French sentence of eight
  words, when the agent stops, then the hook exits 0: the code is not prose and eight words are under the floor.
- (LNG-2) Given an event with no `last_assistant_message` whose transcript ends with an English text block then a
  `tool_use` line of the same message, when the agent stops, then the English block is judged; when a typed message
  follows that block and the turn holds no text, nothing is judged.
- (LNG-3) Given an agent that entered a nested clone with no configuration, when it stops, then the
  `user_language` of the project of `CLAUDE_PROJECT_DIR` is read.
- (LNG-9) Given `user_language = "French"` and a transcript whose last assistant message holds 30 English words then
  a `Bash` call, when the call returns, then the hook prints the added context; given a second call of the same turn
  after that hint, it prints nothing (LNG-8); given a new typed message then an English block and a call, it hints
  again.
- (LNG-7) Given a message holding an English text block then two parallel `tool_use` blocks, when each call returns,
  then only the first call judges the text; when the text block is followed by a tool result of an earlier call
  before its own call, only the text after that result is judged.

## Non-goals

- Languages other than French and English: the stop-word lists of BRE-10 only know those two.
- Rewriting the text blocks already shown: the `PostToolUse` hint asks for the next texts, the user read the others.
- A second `PostToolUse` hint in a turn after the first, even when later blocks breach again: a hint on every call of
  a long turn floods the context.
- The text written by a sub-agent: the user does not read it, and its calls are not in the session's transcript.
- Judging again after another Stop hook sent the agent back in the same turn: Claude Code sets `stop_hook_active`
  for every hook, and the hook lets go to avoid a loop.
- Rewriting the reply for the agent or quoting it in the hint: the agent has it in context.
