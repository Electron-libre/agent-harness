# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.
- [fix_commit_detection_in_the_attribution_hook](fix_commit_detection_in_the_attribution_hook.md) - before run_agents_in_parallel: small and independent, closes a hole in a hook that is already live
- [run_agents_in_parallel_on_different_issues](run_agents_in_parallel_on_different_issues.md) - after audit_agent_instructions: independent, but its queue handling is easier to settle once the audit's design is chosen
