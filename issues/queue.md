# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.
- [hook_code_review_before_commit](hook_code_review_before_commit.md) - after the attribution hook: same mechanism (a `Bash` hook on `git commit`), harder because it needs state
- [run_agents_in_parallel_on_different_issues](run_agents_in_parallel_on_different_issues.md) - after audit_agent_instructions: independent, but its queue handling is easier to settle once the audit's design is chosen
