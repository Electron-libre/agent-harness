# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.
- [make_the_review_waiver_usable](make_the_review_waiver_usable.md) - the documented waiver fails in the worktrees the harness itself creates
- [harden_finish_issue_and_next_issue](harden_finish_issue_and_next_issue.md) - finish removes the current directory; a multi-commit squash keeps an issue taken
- [judge_the_repository_that_commits_in_the_review_hook](judge_the_repository_that_commits_in_the_review_hook.md) - a `cd` before `git commit` is not followed; the native gate still catches it
- [limit_the_spec_checker_skipped_directories](limit_the_spec_checker_skipped_directories.md) - small and safe, but only hits unluckily named directories
