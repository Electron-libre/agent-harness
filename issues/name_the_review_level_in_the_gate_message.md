# Name the review level in the commit gate's refusal

# Goal
When the commit gate refuses a commit, its message tells the agent to run `/code-review` with no level, so the
review reuses whatever level was typed last. `issue-workflow` now names the level of each review. The refusal should
give the same instruction as the skill.

# Findings
* `files/.claude/hooks/require_code_review.rs`, `refuse()`: "Run /code-review on the current diff with the Skill tool
  (issue-workflow, step 4.5), fix what it finds, then commit."
* `files/.claude/skills/issue-workflow/SKILL.md` step 4.5 now says `/code-review low` per deliverable and "always
  name the level"; a `high` review runs once per issue on `<base>..HEAD`.
* Found by the end-of-issue `high` review of `tighten_the_issue_workflow` (finding outside its deliverables).

# Approach
1. Make the refusal say `/code-review low` (the level of a deliverable), keeping the pointer to step 4.5. No test
   nor spec pins the text today.

# Resources
* `files/.claude/hooks/require_code_review.rs`
* `files/.claude/skills/issue-workflow/SKILL.md` (step 4.5)
