# Let the user ask `issue-workflow` to run autonomously

# Goal
When the user says to continue autonomously, `skill:issue-workflow` should chain the deliverables without the
validation stops of steps 1 to 3 and 6, and keep only the stops that protect the user: anything outward-facing,
a blocking question, and the final summary.

# Findings
* Found by the instruction audit (counts only): about ten messages over ten sessions and four projects ask the
  agent to proceed autonomously "to the end" after validating a plan.
* Today the skill says "never skip a validation stop" and the agent has to be told again each time which
  stops are lifted, and whether the review and the tests still apply.

# Approach
Propose, to decide at `issue-workflow` steps 1 to 3:
1. A short section of `issue-workflow` defining the autonomous mode: triggered by an explicit user request,
   recorded in the issue's `## Decisions`, it lifts steps 4.6 (human review) and the commit validation but never
   the review, the checks, or the rules on push and deletion.
2. Whether the mode survives a `/clear` or a resumed thread (written in `# Tracking`, so yes).

# Resources
* `files/.claude/skills/issue-workflow/SKILL.md`

# Hints
* The skill tests (`test_skills.rs`) check the references of a skill: keep them valid.
