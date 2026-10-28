# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [rename_the_consumer_files_to_cardinal](rename_the_consumer_files_to_cardinal.md) - after the binary: its migration of the old names ships in `cardinal`
- [rename_the_harness_named_tools_to_cardinal](rename_the_harness_named_tools_to_cardinal.md) - last of the rename: the leftovers once the binary and files carry the new name
- [drop_the_harness_alias](drop_the_harness_alias.md) - after the rename: only once every consumer runs `cardinal`
- [measure_the_effect_of_the_language_hint](measure_the_effect_of_the_language_hint.md) - last: needs sessions run with the hook before it can measure anything
