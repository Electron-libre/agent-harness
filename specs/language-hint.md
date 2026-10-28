---
id: LNG
status: accepted
adrs: [0005]
---

# Language hint

## Intent

The Stop hook `cardinal run language-hint` sends the agent back once when its last reply is not in the
`user_language` of `.claude/harness.toml`, a rule the agent breaks although it is always in its context. It judges
the reply with the test of rule `user-language` (BRE-10, `specs/rule-breaches.md`), so the hook and the measure agree
on what a breach is.

## Requirements

- **LNG-1** (deterministic) The hook must exit 0 and print nothing when the event sets `stop_hook_active`.
- **LNG-2** (deterministic) The hook must judge the `last_assistant_message` of the event, and, when the event has none, the text blocks of the last assistant line holding text in the transcript at its `transcript_path`, joined by a blank line, provided no message the user typed (the filter of `transcripts.rs`) comes after that line.
- **LNG-3** (deterministic) The hook must read `user_language` in the `[project]` table of the `.claude/harness.toml` at the root of the work tree holding the `cwd` of the event, and in that of `CLAUDE_PROJECT_DIR` when the event has no `cwd`, it is outside a git work tree or that work tree has no `.claude/harness.toml`.
- **LNG-4** (deterministic) The hook must judge the text with the same function as BRE-10: at least 20 words once inline code and fenced code blocks are removed, and more stop words of the other language than of `user_language`.
- **LNG-5** (deterministic) When the text breaks the test of LNG-4, the hook must exit 2 with exactly one line on stderr that names `user_language` and asks to give the reply again in it, quoting no text of the reply.
- **LNG-6** (deterministic) The hook must exit 0 and print nothing when the text passes the test of LNG-4, holds fewer than 20 words of prose, or cannot be judged (an unreadable event, transcript or config, no `user_language`, a language with no stop words).

## Scenarios

- (LNG-5) Given `user_language = "French"` and a last reply of 30 English words, when the agent stops, then the hook
  exits 2 with the hint; when the agent stops again (`stop_hook_active`), it exits 0 (LNG-1).
- (LNG-6) Given `user_language = "French"` and a reply made of a fenced Rust block and one French sentence of eight
  words, when the agent stops, then the hook exits 0: the code is not prose and eight words are under the floor.
- (LNG-2) Given an event with no `last_assistant_message` whose transcript ends with an English text block then a
  `tool_use` line of the same message, when the agent stops, then the English block is judged; when a typed message
  follows that block and the turn holds no text, nothing is judged.
- (LNG-3) Given an agent that entered a nested clone with no `.claude/harness.toml`, when it stops, then the
  `user_language` of the project of `CLAUDE_PROJECT_DIR` is read.

## Non-goals

- Languages other than French and English: the stop-word lists of BRE-10 only know those two.
- Judging the intermediate text blocks of the turn (between tool calls): only the reply the user reads last.
- Judging again after another Stop hook sent the agent back in the same turn: Claude Code sets `stop_hook_active`
  for every hook, and the hook lets go to avoid a loop.
- Rewriting the reply for the agent or quoting it in the hint: the agent has it in context.
