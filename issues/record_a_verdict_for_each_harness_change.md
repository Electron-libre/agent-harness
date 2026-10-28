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
* `specs/harness-verdicts.md` (VRD-1 to VRD-8, new); `specs/harness-report.md` (REP-3 amended for `--changes`).

# Resources
* `files/.claude/skills/evaluate-harness/SKILL.md`, `specs/harness-report.md`, `specs/delivered-quality.md`
* `AGENTS.md` (commit message format), `crates/harness/src/tools/harness_report.rs`
* `https://github.com/google-research/harness2`, `harness2/core/prompts.py`

# Hints
* No target on agent behaviour: the expectation names a direction, never a threshold to reach.
* After `weigh_the_form_and_the_evidence_of_harness_proposals`, which sets the evidence rules this verdict applies.

# Tracking

Plan: validated

## Understanding
A `feat(files)`/`fix(files)` commit records `Expected-effect: <metric> <up|down>` (a column of `harness-report`);
`harness run harness-changes` lists them with the revisions in use that hold each change, and `harness-report
--changes` gives each a verdict from the groups around it, `confounded` by default. Out of scope: replays,
thresholds, expectations outside the commit messages. Spec: `specs/harness-verdicts.md` (VRD), REP-3 amended.

## Plan
1. [x] Record the expectation: `Expected-effect` line checked by `commit-check`, `harness run harness-changes`
   (rows of the expectations and the revisions holding each change); verified by tests through the binary on a
   temp git repository; makes true VRD-1, VRD-2, VRD-3, VRD-4, VRD-7, VRD-8; ~600 lines.
2. [x] Verdict table: `harness-report --changes` prints `## Verdicts` (groups before and after, values, verdict);
   verified by tests through the binary on TSV fixtures; makes true VRD-5, VRD-6, REP-3 (amended); ~500 lines.
3. [ ] Skill and docs: `evaluate-harness` collects `harness-changes`, passes `--changes` and reads the verdicts
   (`confounded` and `too few` are no conclusion); `AGENTS.md` names the footer line; verified by new
   `REQUIRED_PASSAGES` in `crates/harness/tests/skills.rs`, `just check-harness`; ~80 lines.

## Decisions
- Mode: autonomous
- Base commit: 0de9d401e04936ce272bd199708b04844399a362
- Where: the commit message (a footer line), read from the history of the harness source; no tool reads an issue
  section or a `docs/` file for this.
- Confounders (VRD-6): the most frequent name of `models`, `versions` and `projects` is compared, not the whole set
  (a weekly Claude Code release would confound everything); any other change with an expectation and the same after
  group confounds, whatever its metric.

## Deliverable 1
done: `Expected-effect: <metric> <up|down>` lines checked by `commit-check`; `harness run harness-changes [repo]
--sessions <file>` lists them per change of `files/` with the revisions in use that hold it.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass; VRD draft; REP-2, 3, 6, 7 warn of 2 `must` since REP is back in `draft` (pre-existing wording) |
| docs | `docs/usage.md`, `docs/architecture.md` (reader table) |
| code review | low, see below |

red: VRD-3, VRD-4, VRD-7, VRD-8: `harness: unknown tool 'harness-changes'`; VRD-1, VRD-2: `unknown metric:
left: 0, right: 1` (`commit-check` accepted `Expected-effect: speed up`).

Code review: no findings

not verified: a run on the real history (no commit carries an expectation yet).

decisions needed: none

## Deliverable 2
done: `harness-report --changes <file>` prints a last `## Verdicts` table (groups before and after, values,
verdict); the axis tables return their cells, computed for a `too few` group too; VRD and REP back to `accepted`.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass; VRD-5 says the values of a `too few` group are computed (so it reads `confounded`, not `no data`) |
| docs | `docs/usage.md`, `docs/architecture.md` (diagram, reader table) |
| code review | low, see below |

red: VRD-5, VRD-6, VRD-7: the 6 verdict checks failed (`unknown argument --changes`, no `Verdicts` table).

Code review: 1 finding, not fixed: the skills table is left out of the verdict lookup, which no VRD-1 metric
reads (`calls`, `hook_refusals`, `unused` are no metric).

not verified: a report on real transcripts with a recorded change (none recorded yet).

decisions needed: none
