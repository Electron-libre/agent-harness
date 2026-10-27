---
id: EVA
status: draft
adrs: [0002, 0003, 0006, 0008]
---

# Evaluation of the harness from the session transcripts

## Intent

`.claude/transcripts/session_metrics.rs` turns the Claude Code transcripts of every project into one row of
numbers per session (cost, user interventions, and what may explain a difference: harness revision, model, Claude
Code version, project, issue), so the other evaluation tools compare sessions without reading the transcripts. It
measures; it does not judge.

## Requirements

- **EVA-1** (deterministic) `session_metrics.rs [projects_dir]` must print a header line then one tab-separated row per session (one `<session>.jsonl` of a project directory, default `~/.claude/projects`), with the columns `session`, `project`, `issues`, `revision`, `models`, `versions`, `start`, `duration_s`, `turns`, `tool_calls`, `input_tokens`, `output_tokens`, `thinking_tokens`, `cache_read_tokens`, `cache_creation_tokens`, `hook_refusals`, `skills`, `validations`, `other_messages`, `unreadable_lines`, `unknown_lines`, `missing_fields`.
- **EVA-2** (deterministic) An assistant message must be counted once per `message.id`, whatever the number of lines that repeat it: one turn, its `message.usage` tokens added once (`thinking_tokens` from `output_tokens_details.thinking_tokens`, 0 when absent), its model counted once; `tool_calls` counts the `tool_use` blocks of all its lines.
- **EVA-3** (deterministic) The transcripts of a session's sub-agents (`<session>/subagents/*.jsonl`) must be added to that session's row (turns, tool calls, tokens, hook refusals, skills, skipped lines), and must not produce a row of their own nor add typed messages.
- **EVA-4** (deterministic) A typed message (the same filter as `audit_sources.rs`: a `user` line, not meta, not sidechain, not a tool result nor injected text) must count in `validations` when, trimmed, lower-cased and stripped of trailing `.` and `!`, it is one of `ok`, `okay`, `yes`, `y`, `oui`, `go`, `continue`, `lgtm`, `vas-y` or `c'est bon`, or answers the questions of a stop (only comma-separated items of a number then one letter from `a` to `h`, such as `1 a, 2 c`: two items at least, or one with a space, so `4k` is not one); any other typed message counts in `other_messages`.
- **EVA-5** (deterministic) A tool result with `is_error` whose text starts with `PreToolUse:<tool> hook error:` (`<tool>` with no space) must count one refusal for the hook named by the stem of the first `.rs` file name its text cites (`.rs` ending the name, so `.rsync` or `.rst` is not one) (`unknown` when none); `hook_refusals` lists `name=count` pairs joined by `;`, sorted by name. `skills` lists the `Skill` tool calls the same way, by `input.skill`.
- **EVA-6** (deterministic) `models` and `versions` must list the distinct `message.model` and `version` values of the session joined by `,`, in order of first appearance; `issues` the distinct `gitBranch` values other than `main`, `master` and `HEAD`, the same way (empty when none: a session is not forced onto an issue); `start` is the earliest `timestamp` and `duration_s` the whole seconds to the latest.
- **EVA-7** (deterministic) `project` must be the name of the directory holding the git common directory of the first `cwd` of the session (so a linked worktree counts with its main repository), the work tree of a submodule or of a separate git directory being its own project. When the `cwd` no longer exists and is under `<repo>/.claude/worktrees/` of an existing repository `<repo>` (a removed issue or agent worktree, ADR 0008), or its name is `<repo>-<branch>` next to an existing repository `<repo>` (a removed issue worktree of ADR 0003, `<branch>` being a `gitBranch` of the session), that repository is the project; otherwise the last component of that `cwd`.
- **EVA-8** (deterministic) `revision` must be the `rev` value of the `.claude/harness.toml` committed on the default branch of the project at the session's `start` (the last commit of its first-parent history touching that file at or before it, so a commit merged later from another branch does not count); `self:<commit>` when that file names `source = "."` with no `rev` (the harness repository itself: the last commit touching `files/` at or before `start`); `unpinned` when the file exists then with no `rev`; empty when the project, the file or the history cannot be read.
- **EVA-9** (deterministic) A line that is not valid JSON must count in `unreadable_lines`, a line whose `type` is not in the known list of the script in `unknown_lines`, and an assistant line with no `message.id` or no `message.usage`, or a line of the conversation (`user`, `assistant`, `attachment`, `system`) with no `timestamp`, in `missing_fields` (the metadata lines, such as `ai-title`, carry none); none of them must stop the script or change the other columns.
- **EVA-10** (deterministic) The output must hold no text of a message, of a tool input or of a tool result: only identifiers, names, counts and timestamps.
- **EVA-11** (deterministic) With `--archive <file>`, the script must also merge its rows into that file: a session already archived is replaced by its new row, a session absent from the transcripts is kept, and the file stays one header then rows sorted by `start`; the parent directory is created when missing. An archive whose header is not the script's must be left unchanged, with an error and a non-zero exit, since its columns would not line up.
- **EVA-12** (deterministic) `audit_sources.rs` and `session_metrics.rs` must read the transcripts through the same module (`.claude/transcripts/transcripts.rs`): the typed-message filter and the listing of projects and sessions are written once.
- **EVA-13** (deterministic) `session_metrics.rs` and `audit_sources.rs` must each carry the line `// transcripts.rs sha256: <hex>` with the SHA-256 of the current `transcripts.rs`, so that a change of the module changes the text of both scripts and `rust-script` rebuilds them (it watches only the script file); `just test-harness` must fail, giving the line to write, when one is stale.
- **EVA-14** (deterministic) When `--archive` replaces a row whose `revision` was not empty with a new row whose `revision` is empty (the repository is gone or unreadable), the merged row must keep the archived `project` and `revision`.

## Scenarios

- (EVA-2) Given an assistant message split over three lines with the same `message.id`, each carrying the same usage and one `tool_use`, when the row is printed, then `turns` is 1, the tokens are those of one usage and `tool_calls` is 3.
- (EVA-8) Given a project whose `harness.toml` moved from `rev = "a"` to `rev = "b"` in a commit at 10:00, when a session starts at 09:00, then its revision is `a`; at 11:00, `b`.
- (EVA-8) Given a session run in a linked worktree on a branch that changed the pin, when the revision is read, then it is the pin of the default branch, since the branch may be gone.
- (EVA-11) Given an archive holding sessions A and B, when the transcripts hold only B (changed) and C, then the archive holds A, the new B and C.

## Non-goals

- Judging a session (good, bad, rule broken): the detectors and the report are other issues.
- The price in money of the tokens: models and prices change; the tokens by kind are enough to price later.
- A complete list of the line types: an unknown type is counted, not an error.
- Counting the slash commands the user types (`<command-name>…`): the typed-message filter drops them, so they count in neither `validations` nor `other_messages`; measured at 0.3 % of the typed messages on 2026-10-26 (`issues/done/count_typed_slash_commands_as_interventions.md`), a `slash_commands` column by command name is the candidate if they become a usual way to steer the work.
