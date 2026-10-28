# Measure the effect of the language hint

# Goal
The `language-hint` Stop hook (`specs/language-hint.md`) sends the agent back once when its last reply is not in the
`user_language`. Measure whether it lowers the share of sessions that breach rule `user-language`, then keep the hook
or drop it.

# Findings
* `harness run rule-breaches --summary` on 2026-10-28, before the hook: rule `user-language` breached in 12 of the
  142 sessions where it applied, 21 breaches.
* The hook was added in commit `ad56d2a`; no session had run with it when this issue was written.
* The hook judges only the last reply of a turn, while BRE-10 judges every assistant text block: intermediate blocks
  between tool calls can still count as breaches.

# Approach
1. Wait until enough sessions ran with a revision holding the hook (`skill:evaluate-harness` gives the sessions per
   revision).
2. Compare `breached` / `applied` of `user-language` before and after that revision, with the model and Claude Code
   version next to each number.
3. Candidate outcomes: keep the hook; also judge the intermediate blocks; drop it when the share does not move.

# Resources
* `specs/language-hint.md`, `specs/rule-breaches.md` (BRE-10)
* `crates/cardinal/src/tools/language_hint.rs`, `crates/cardinal/src/tools/language.rs`
* `skill:evaluate-harness`

# Hints
* A breach the hook caught and the agent then fixed still shows in the transcript as a breach of BRE-10: count the
  final replies apart, or the hook looks useless.
* Found by the review of the hook (2026-10-28): the English list of BRE-10 holds `an` and `as`, also French words
  (`un an`, `tu as`), and `Language::named` knows only the exact strings `French` and `English` (`french` turns the
  rule off silently). Both skew the measure and the hook alike.
