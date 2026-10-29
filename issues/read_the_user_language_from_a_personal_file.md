# Read the user language from a personal file

# Goal
`user_language` is a preference of the person who talks to the agent, yet it sits in the `[project]` table of
`.claude/cardinal.toml`, a file the project commits and shares. In a team every member gets the same conversation
language. Read it (and any later personal preference) from a file each person owns and git ignores, the shared value
staying the default.

# Findings
* Readers of `user_language`: the `language-hint` Stop hook (`crates/cardinal/src/tools/language_hint.rs`, LNG-3),
  `Language::from_config` (`crates/cardinal/src/tools/language.rs`), rule `user-language` of `rule-breaches`
  (BRE-10), and the skills `issue-workflow`, `prioritize-issues`, `create-issue`, `audit-instructions` and
  `evaluate-harness`, which tell the agent to read the key in `.claude/cardinal.toml`.
* BRE-10 reads the `.claude/cardinal.toml` committed at the session's `start`: a git-ignored file has no history, so
  the rule would judge a session against the shared value, not the language that session used.
* `seed/.claude/cardinal.toml` seeds `user_language = "English"`; no other key of `[project]` is personal today.
* The user agreed on 2026-10-29 that this is worth doing for a team; the harness itself stays imposed on every
  member (the review gate is intended).

# Approach
1. Choose the personal file: a git-ignored `.claude/cardinal.local.toml` whose `[project]` values override the shared
   ones, or the `env` of Claude Code's personal `.claude/settings.local.json`, or a user-level file outside the
   repository (one language for every project of a person).
2. One reader for the merged configuration, used by the hook and `rule-breaches`, instead of each tool reading the
   key; `sync` seeds or ignores the personal file (an entry in `.gitignore`).
3. For BRE-10: read the personal value as it is when the measure runs, or record the language of the session in
   the transcript (the hint's own output), or keep the committed value as the only measured one and say so.
4. Point the skills at the merged value instead of `.claude/cardinal.toml`.

# Specs
* `specs/language-hint.md` (LNG-3), `specs/rule-breaches.md` (BRE-10), `specs/sync.md` if `sync` seeds or ignores
  the personal file.

# Resources
* `docs/architecture.md` ("What is shared, and how": project values go through `.claude/cardinal.toml`).
* `crates/cardinal/src/tools/language.rs`, `language_hint.rs`, `rule_breaches.rs`.
* `issues/done/hint_when_the_reply_is_not_in_the_user_language.md`.

# Hints
* A personal file must never be managed: `sync` and `check` hash only what the source writes.
* Keep the shared value as the fallback, so a solo project changes nothing.
