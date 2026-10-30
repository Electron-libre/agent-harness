---
rank: 58
why: 'before the exploration verdict, which reads input_tokens: cheap, from columns already collected, and makes every token verdict tell a cache read from a full-price input'
---
# Show the cache read share in the efficiency table

# Goal
The `input_tokens` column of the efficiency table adds up the uncached input, the cache reads and the cache writes
(REP-5). A cache read is billed about a tenth of an uncached input and a cache write more than one, so a change that
breaks the cache and one that grows the context look alike, and a verdict on `input_tokens` cannot tell a cheaper
session from a costlier one. The expected outcome: the efficiency table shows how much of the input came from the
cache, and the verdicts can read it.

# Findings
* `session-metrics` already prints `input_tokens`, `cache_read_tokens` and `cache_creation_tokens` per session
  (EVA-1); only the report merges them (REP-5).
* The token-waste literature names an unstable prompt prefix (a timestamp, a changing order) among the main causes
  of waste; Cardinal measures the size of the fixed context (`fixed-context`, REP-7) but not whether it is read from
  the cache.

# Approach
1. A `cache_read_share` column: per group, the median over its sessions of `cache_read_tokens` over the sum of the
   three input columns, as a whole percent.
2. Or the three input columns kept apart (medians of each), the sum staying for continuity.
3. Optionally, a cost-weighted input (cache read and write weighted by their price ratio to an uncached input): the
   ratios are prices of the provider and may change, so a column with fixed weights may age.
Then the metric for `Expected-effect:` (VRD-1) and its verdict.

# Specs
* `specs/cardinal-report.md`: amends REP-5 or adds a requirement for the new column.
* `specs/cardinal-verdicts.md`: VRD-1 gains the metric.

# Milestone
Lean agent loop

# Resources
* `specs/cardinal-report.md` (REP-5, REP-9 to REP-11: how a column was added last), `specs/cardinal-verdicts.md`.
* `crates/cardinal/src/tools/cardinal_report.rs`, `session_metrics.rs`.

# Hints
* A ratio of medians is not the median of ratios: say which one the column is.
