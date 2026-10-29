# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [judge_the_text_blocks_between_tool_calls](judge_the_text_blocks_between_tool_calls.md) - 90% of the language breaches are in the blocks the language hint never judges; before the CI installer gap, which no consumer hits today
- [fail_the_ci_installer_when_the_configuration_is_missing](fail_the_ci_installer_when_the_configuration_is_missing.md) - a pre-existing gap of the CI install, no consumer lacks its configuration today
- [trace_the_specs_in_the_tests_of_any_language](trace_the_specs_in_the_tests_of_any_language.md) - first of the other stacks, which no consumer has yet: without it the spec workflow and the quality measures fail outside Rust; before the team issues, which are comfort
- [run_the_lint_and_format_of_the_project_stack](run_the_lint_and_format_of_the_project_stack.md) - after the tracing: a missing lint gate leaves a project working, a missing tracing does not
- [seed_no_cargo_permissions_in_a_project_of_another_stack](seed_no_cargo_permissions_in_a_project_of_another_stack.md) - last of the other stacks: the cost is only permission prompts
- [read_the_user_language_from_a_personal_file](read_the_user_language_from_a_personal_file.md) - for team use, which no project has yet; after the judge of the text blocks, which changes the same language rule
- [rank_the_issues_without_a_shared_queue_file](rank_the_issues_without_a_shared_queue_file.md) - last: for team use, and its risk of conflict is not measured yet
