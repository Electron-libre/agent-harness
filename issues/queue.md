# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [move_tracked_issues_with_git_mv](move_tracked_issues_with_git_mv.md) - only open issue: every close needs a manual `git add` until it is fixed
- [refuse_closing_an_issue_of_a_linked_worktree_by_path](refuse_closing_an_issue_of_a_linked_worktree_by_path.md) - after move_tracked_issues_with_git_mv, which reworks the same `main`: closes a PAR-8 bypass
