# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.
- [audit_agent_instructions](audit_agent_instructions.md) - new feature, depends on nothing open; the queue was empty
- [hook_block_ai_attribution_in_commits](hook_block_ai_attribution_in_commits.md) - small, mechanical and the rule is already broken in practice; independent of the other open issues
- [hook_code_review_before_commit](hook_code_review_before_commit.md) - after the attribution hook: same mechanism (a `Bash` hook on `git commit`), harder because it needs state
- [issue_workflow_autonomous_mode](issue_workflow_autonomous_mode.md) - small edit of one skill, no dependency; ranked after the hooks
- [run_agents_in_parallel_on_different_issues](run_agents_in_parallel_on_different_issues.md) - after audit_agent_instructions: independent, but its queue handling is easier to settle once the audit's design is chosen
