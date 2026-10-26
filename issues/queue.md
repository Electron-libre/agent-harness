# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [drop_redundant_guards_and_prose_tests](drop_redundant_guards_and_prose_tests.md) - removes the wording tests that would otherwise trip on the workflow rewrite
- [tighten_the_issue_workflow](tighten_the_issue_workflow.md) - scopes and levels the review once the gate and prose tests are slimmed
- [make_the_checks_quiet_and_cheap](make_the_checks_quiet_and_cheap.md) - output savings, independent and smaller, measure first
