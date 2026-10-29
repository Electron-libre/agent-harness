# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [measure_the_effect_of_the_language_hint](measure_the_effect_of_the_language_hint.md) - last: needs sessions run with the hook before it can measure anything
- [judge_the_text_blocks_between_tool_calls](judge_the_text_blocks_between_tool_calls.md) - after the measure of the language hint, which found 90% of the language breaches in the blocks it never judges; before the CI installer gap, which no consumer hits today
- [fail_the_ci_installer_when_the_configuration_is_missing](fail_the_ci_installer_when_the_configuration_is_missing.md) - last: a pre-existing gap of the CI install, no consumer lacks its configuration today
