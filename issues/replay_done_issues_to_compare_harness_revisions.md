# Replay done issues to compare two harness revisions

# Goal
The transcripts are observational: task, model and Claude Code version change with the harness, so they show
trends, not causes. For a major harness change, compare two revisions on the same tasks with the same model.

# Findings
* `issues/done/` holds closed issues with their base commit (`## Decisions`, "Base commit") and their delivered
  commits: a task, its starting point and a reference solution.
* One worktree per issue (ADR 0003) already isolates parallel agents; `harness sync` can pin a revision (`rev`,
  ADR 0002).
* Another project of the user has a `quality-benchmark` skill (3 calls in the transcripts): an idea to look at, not
  to copy.

# Approach
1. Pick a small fixed set of done issues (to decide: which, how many) that are self-contained.
2. For each harness revision under test: a worktree at the issue's base commit, the harness synced at that
   revision, an agent run in non-interactive mode with a fixed model and a scripted user that validates the stops.
3. Compare with the metrics and quality extractors of the previous issues, plus whether the run's tests pass
   against the reference tests of the delivered commits.
4. Run several times per revision to see the variance before reading a difference.

# Resources
* `docs/adr/0002-rev-pins-the-resolved-commit.md`, `docs/adr/0003-one-worktree-per-issue.md`
* `files/.claude/skills/issue-workflow/SKILL.md` (autonomous mode, validation stop)
* `issues/done/report_the_harness_evolution_by_revision.md`

# Hints
* Costly: each run is a full agent session; keep the set small and run it only around a major change.
* A replayed issue may be in the model's context already through the repository history: start from the base
  commit with the later history out of reach.
* No network in unit tests: the replay is a manual recipe, its parsing code is what gets tested.
