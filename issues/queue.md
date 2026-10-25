# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.
- [pick_the_reviewed_repository_from_git_dir_and_work_tree](pick_the_reviewed_repository_from_git_dir_and_work_tree.md) - after the waiver issue: a remaining bypass of the review gate, same hook and test file
- [handle_squash_merged_issue_branches](handle_squash_merged_issue_branches.md) - blocks the cleanup of issues merged by squash
- [review_the_attribution_hook_scan_scope](review_the_attribution_hook_scan_scope.md) - false positives and a possible miss, to reproduce first
- [read_git_dir_and_git_work_tree_in_the_review_hook](read_git_dir_and_git_work_tree_in_the_review_hook.md) - after the attribution hook issue: a narrower bypass of the review gate, follows the target selection of `REV-1` to `REV-5`
- [trace_managed_test_citations_in_the_harness_repository](trace_managed_test_citations_in_the_harness_repository.md) - low: a gap accepted on purpose, after the open hook issues
- [cheapen_the_commit_gate_hooks](cheapen_the_commit_gate_hooks.md) - last: performance, to measure before acting
