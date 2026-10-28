# Delegate the exploration of the issue workflow to a subagent

# Goal
The main session of `skill:issue-workflow` reads the code, docs and history itself at step 1 and step 3.1, and
everything it reads stays in its context until the end of the issue. That growth is what the size limit of step 1
(~3,000 lines, 6 deliverables per issue) guards against. A read-only subagent (the built-in `Explore` type) would
return only its conclusion, so the main context would grow more slowly and an issue could keep its quality longer.
The expected outcome: the skill tells the agent when to delegate a broad exploration, and the evaluation can tell
whether that made the sessions cheaper or not.

# Findings
* The workflow already delegates one step: `code-review` runs forked, and the review gate records its marker on the
  `PostToolUse` of the `Skill` tool (`crates/harness/src/tools/review_gate.rs`, module doc).
* `session-metrics`, `rule-breaches` and `delivered-quality` already read the subagent transcripts of a session
  (`subagents(&path)`).
* `session-metrics` adds the subagent transcripts to their session (`session.read(&sub, false)`): `input_tokens` and
  the cache columns are the sum of the main thread and its subagents. No metric shows the size of the main context,
  which is the benefit expected here; a delegation could even raise the summed tokens, since each subagent starts
  cold and rereads the instructions.
* No skill of `files/` mentions a subagent today; the repository ships no `.claude/agents/` definition.

# Approach
Measure first: the current metrics cannot show the expected benefit.

1. Add a metric of the main thread only to `session-metrics` (e.g. the peak of the input and cache tokens of one
   turn of the main transcript, or the main-thread tokens apart from the subagents' ones), so a verdict can read it.
2. In `issue-workflow` step 1 and step 3.1, tell the agent to hand a broad exploration (many files, history,
   conventions) to an `Explore` subagent and to keep only its conclusion; a targeted read of a known file stays in
   the main thread. No custom agent definition, so the fixed context does not grow.
3. Commit the skill change with `Expected-effect:` lines (the new metric `down`, and `input_tokens` if it should
   move) and let `skill:evaluate-harness` give it a verdict once enough sessions ran.

Left out unless the verdict is good: a subagent writing the tests of step 3.2 from the spec alone, a subagent
running `skill:update-docs`. The spec, the plan, the implementation and the commit message stay in the main thread
(they need the dialogue with the user and the recorded decisions).

# Specs
* `specs/evaluation.md`: a new requirement for the main-thread metric (1); `specs/harness-report.md` (REP-5) to show it, and the `<metric>` list of VRD-1 in
  `specs/harness-verdicts.md` if the metric is to be named in an `Expected-effect:` line.

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 1 and 3.1, size limit).
* `crates/harness/src/tools/session_metrics.rs`, `crates/harness/src/tools/transcripts.rs` (`subagents`).
* `specs/evaluation.md`, `specs/harness-verdicts.md`, `files/.claude/skills/evaluate-harness/SKILL.md`.
* `docs/adr/0006-transcript-readers-are-managed-scripts.md`.

# Hints
* A review done by anything other than the `code-review` skill leaves no marker: the commit gate refuses the commit.
  Keep the skill as the entry point of every review.
* The `red:` and `Code review:` lines that `delivered-quality` counts must stay written by the main thread in the
  issue.
* Only read-only subagents are safe in parallel: two that write in the same worktree conflict.
* A wrong or partial conclusion of the subagent is taken as is by the main thread, which never saw the raw files:
  the instruction should ask for the paths and lines behind each conclusion.
