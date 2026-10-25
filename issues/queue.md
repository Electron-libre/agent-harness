# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.
- [match_git_value_options_in_the_review_hook](match_git_value_options_in_the_review_hook.md) - next: a bypass of the review gate, small fix
- [keep_the_review_waiver_until_the_commit_happens](keep_the_review_waiver_until_the_commit_happens.md) - after the bypass: same hook, same test file
- [handle_squash_merged_issue_branches](handle_squash_merged_issue_branches.md) - blocks the cleanup of issues merged by squash
- [review_the_attribution_hook_scan_scope](review_the_attribution_hook_scan_scope.md) - false positives and a possible miss, to reproduce first
- [cheapen_the_commit_gate_hooks](cheapen_the_commit_gate_hooks.md) - last: performance, to measure before acting
