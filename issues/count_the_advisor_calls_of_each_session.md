---
rank: 25
why: 'first: nothing on the advisor can be judged before its use is counted; the advisor issues after it rest on it'
---
# Count the advisor calls of each session

# Goal
Claude Code can pair the session's model with a stronger advisor model (`advisorModel` setting, `/advisor`,
`--advisor`), which the executor consults mid-turn. The evaluation of Cardinal cannot tell today whether a session had
an advisor, how often it consulted it, nor what it cost. The outcome: `session-metrics` gives per session the advisor
model and its calls (and its tokens if the transcript has them), so the harness changes that steer the advisor can get
a verdict.

# Findings
* Measured on the user's machine (2026-10-30, counts only): the `assistant` lines of the transcripts carry a top-level
  `advisorModel` field (about 44,000 lines with an Opus advisor).
* No content block named `advisor` nor typed `advisor_*` was found in any transcript, sub-agents included; the
  `usage.iterations[]` of the assistant messages hold only `type: "message"` entries, no `advisor_message`. Unknown
  whether Claude Code never writes the call or the advisor was never called (the main model was mostly the same Opus).
* The API returns the advisor's usage in `usage.iterations[]` (`type: "advisor_message"`), billed at the advisor's
  rate, and its result as an `advisor_tool_result` or `advisor_redacted_result` block.
* `session-metrics` (EVA-1) has `models` but no advisor column; EVA-3 already folds the sub-agent transcripts in.

# Approach
1. Measure first: run a short session with an advisor that is stronger than the main model (e.g. a Sonnet session with
   an Opus advisor) and a prompt that asks for an advisor call, then read what its transcript records.
2. Add the columns that the transcript supports: `advisor_models` (distinct `advisorModel`, as `models`),
   `advisor_calls`, and `advisor_tokens` if the usage iterations carry them; an absent field is an empty cell, not 0.
3. Carry them into the efficiency table of `cardinal-report` and, if a count is reliable, into the metrics of
   `Expected-effect:`.
If the transcript records no call at all, say so in the spec's non-goals and keep only `advisor_models`.

# Specs
* `specs/evaluation.md`: amends EVA-1, new requirements for the advisor columns.
* `specs/cardinal-report.md`: amends REP-5; `specs/cardinal-verdicts.md` VRD-1 if a metric is added.

# Milestone
Advisor

# Resources
* `crates/cardinal/src/tools/session_metrics.rs`, `specs/evaluation.md`, `specs/cardinal-report.md`.
* Advisor tool: https://platform.claude.com/docs/en/agents-and-tools/tool-use/advisor-tool
* Advisor in Claude Code: https://code.claude.com/docs/en/advisor
* Article: https://claude.com/blog/the-advisor-strategy

# Hints
* An advisor less capable than the main model is not attached to the request: the setting alone does not prove an
  advisor was available; read the pair.
* Opus 5.x, Sonnet 5.5, Haiku 5.5, Fable and Mythos advisors return an encrypted result: count the call, never its text
  (EVA output holds no message text).
