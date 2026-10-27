# Record a verdict for each harness change

# Goal
`skill:evaluate-harness` reports trends per harness revision but never ties a trend to the change that should have
caused it, nor says when it cannot. Record, for each change of the shared harness, the effect it is expected to
have, and let the evaluation give it a verdict from the real transcripts, `confounded` by default when something
else changed too.

# Findings
* `files/.claude/skills/evaluate-harness/SKILL.md` step 2: a trend spans two groups or more; when the models,
  versions or projects changed too, the report says the change may come from them. Nothing names which harness
  change a trend is checked against.
* The evaluation stays on real sessions of real projects; replaying a task under two revisions was ruled out
  (a replay has no user, and a fixed task set biases the harness towards it).
* `google-research/harness2` (`harness2/core/prompts.py`, `experience`) writes one row per harness edit:
  component, change, observed difference, verdict `helped`, `hurt`, `no observed effect` or `confounded`, and asks
  to prefer `confounded` whenever several things changed at once or one sample stands behind the difference.

# Approach
1. Where the expected effect is written: a line in the commit body of a `feat(files)`/`fix(files)` commit, a section
   of the issue, or a file under `docs/`; pick the one `delivered-quality` or `harness-report` can read.
2. The metric each expectation names (a column of `harness-report`), so the verdict reads a number, not prose.
3. `evaluate-harness` lists the recorded changes of the compared revisions with their verdict; `confounded` when
   the model, version or project mix changed between the groups, or a group is `too few`.

# Specs
* `specs/harness-report.md` if a tool reads the expectations; new requirement ids decided at `issue-workflow`
  step 1.

# Resources
* `files/.claude/skills/evaluate-harness/SKILL.md`, `specs/harness-report.md`, `specs/delivered-quality.md`
* `AGENTS.md` (commit message format), `crates/harness/src/tools/harness_report.rs`
* `https://github.com/google-research/harness2`, `harness2/core/prompts.py`

# Hints
* No target on agent behaviour: the expectation names a direction, never a threshold to reach.
* After `weigh_the_form_and_the_evidence_of_harness_proposals`, which sets the evidence rules this verdict applies.
