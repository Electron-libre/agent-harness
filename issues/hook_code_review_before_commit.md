# Refuse a commit until the code review has run on the current diff

# Goal
A hook that makes the agent run the code review before it commits, instead of relying on it remembering step 5
of `skill:issue-workflow`.

# Findings
* Found by the instruction audit (counts only): the code review is mentioned in about 40 user messages over 30
  sessions and 5 projects; the user wrote that the agent systematically forgets it and that it is never optional.
* `issue-workflow` already requires it ("mandatory, never skipped") and still gets skipped, so the instruction
  alone does not hold.
* A hook has no memory: it needs a marker (the diff hash at the time of the review) written when the review
  skill ends and compared at `git commit`.

# Approach
Measure first: check which hook events expose the invocation of a skill (`PostToolUse` on `Skill`) and whether
the review result can be seen.
1. A `PostToolUse` hook on `Skill` for the code review writes a marker holding the hash of `git diff HEAD`
   (staged and unstaged) under the project's `.claude/` state, git-ignored.
2. A `PreToolUse` hook on `Bash` for `git commit` recomputes the hash; if it differs from the marker (or no marker
   exists), it exits 2 telling the agent to run the review.
3. Escape hatches to decide at `issue-workflow` steps 1 to 3: a commit of only docs or issues, and an explicit
   user waiver.

# Resources
* `files/.claude/hooks/`, `files/.claude/skills/issue-workflow/SKILL.md` (step 5)
* `seed/.claude/settings.json` (hooks are declared there)

# Hints
* A fix made after the review changes the hash: the rule is "review the final diff", which is what
  `issue-workflow` asks, but the agent re-runs it often; consider accepting fixes after a review that reported
  findings.
* The shared files hold nothing specific to one project.
