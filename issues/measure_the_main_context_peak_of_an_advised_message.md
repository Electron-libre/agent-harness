---
rank: 37
why: 'right after the advisor count: once advisors are called, every advised session skews the peak that the advisor and context verdicts read'
---
# Measure the main context peak of an advised message

# Goal
`main_context_peak` (EVA-16) adds up the `input_tokens`, `cache_read_input_tokens` and `cache_creation_input_tokens`
of the top-level `message.usage`. On a message with an advisor call, that usage adds up every executor `message`
iteration of `usage.iterations`, so the peak comes out about twice the real largest context. The expected outcome:
the peak of an advised session is the size of its largest context, so an advised group and a solo group can be
compared.

# Findings
* Measured on 2026-10-30 (Claude Code 2.1.312, an Opus session with an Opus advisor and one advisor call): the
  top-level usage reads input 4, cache read 28,546, cache creation 10,919, a peak of 39,469; the two `message`
  iterations read (2, 9,012, 10,522) and (2, 19,534, 397), so the largest context is 19,933.
* A message with no advisor call has a single `message` iteration (or none), so its top-level usage is already the
  context size.
* The top-level sums are what is billed, so `input_tokens`, `cache_read_tokens` and `cache_creation_tokens` stay
  right; only the peak is wrong.
* Found by the `high` review of `issues/count_the_advisor_calls_of_each_session.md`.

# Approach
1. When `usage.iterations` holds entries of type `message`, take the largest per-iteration sum of input and cache as
   the context of the message; otherwise keep the top-level sum.
2. Archived rows keep their old peak: say so in the spec (an advised session archived before the fix is overstated).

# Specs
* `specs/evaluation.md`: amends EVA-16.

# Milestone
Advisor

# Resources
* `crates/cardinal/src/tools/session_metrics.rs` (`Session::assistant`), `crates/cardinal/tests/session_metrics.rs`.
* `specs/evaluation.md` (EVA-16, EVA-19).
