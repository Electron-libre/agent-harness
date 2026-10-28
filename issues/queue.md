# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [rename_the_repository_to_cardinal](rename_the_repository_to_cardinal.md) - first of the Cardinal rename: the later ones start from the new source URL
- [name_the_product_cardinal](name_the_product_cardinal.md) - after the repository rename: the docs name the product before its commands and files change
- [rename_the_binary_to_cardinal](rename_the_binary_to_cardinal.md) - after the product name: the consumer files and tools are renamed by the `cardinal` binary
- [rename_the_consumer_files_to_cardinal](rename_the_consumer_files_to_cardinal.md) - after the binary: its migration of the old names ships in `cardinal`
- [rename_the_harness_named_tools_to_cardinal](rename_the_harness_named_tools_to_cardinal.md) - last of the rename: the leftovers once the binary and files carry the new name
- [measure_the_effect_of_the_language_hint](measure_the_effect_of_the_language_hint.md) - last: needs sessions run with the hook before it can measure anything
