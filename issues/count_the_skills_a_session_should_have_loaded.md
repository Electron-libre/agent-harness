---
rank: 100
why: 'after the exploration verdict: cheap, reuses rule-breaches; gives the skill changes a routing signal'
---
# Count the skills a session should have loaded

# Goal
A single agent loop works only if the agent loads the right skill at the right moment. The evaluation counts the skills
called and those never called (REP-7), but not the moments a skill was due and skipped: a commit made without
`commit-message`, an issue worked without `issue-workflow`, a spec written without `write-spec`. The outcome: such
misses are findings of `rule-breaches`, so a change of a skill description or of the instructions gets a verdict on its
routing.

# Findings
* `rule-breaches` already judges rules from the transcripts and the git history (BRE-4 to BRE-10) and feeds the rules
  table of `cardinal-report` (`applied`, `breached`).
* REP-7 lists `unused` skills per group: a skill never called, not a skill missed when due.
* The article on folding subagents recommends checking, in the evals, that the right skill or tool was loaded.

# Approach
1. Pick the triggers that are deterministic from a transcript: a `git commit` call with no `commit-message` skill call
   before it in the session; a `Plan: proposed` write with no `issue-workflow` call; a new file under `specs/` with no
   `write-spec` call.
2. One rule per trigger (`skill-routing-<skill>` or one rule with the skill as detail), `applied` when the trigger
   occurs.
Measure first how often each trigger occurs; drop a trigger that never fires.

# Specs
* `specs/rule-breaches.md`: new requirements, one per routing rule.

# Milestone
Lean agent loop

# Resources
* `specs/rule-breaches.md`, `crates/cardinal/src/tools/` (the `rule_breaches` module), `specs/cardinal-report.md` REP-6.

# Hints
* A user who commits by hand or asks for a quick fix legitimately skips a skill: the rule is a signal, never a gate.
