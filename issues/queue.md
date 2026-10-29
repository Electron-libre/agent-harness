# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [settle_the_clippy_pedantic_follow_ups](settle_the_clippy_pedantic_follow_ups.md) - after the regression: edge cases and cleanups of the same commit, none observed in use
- [normalize_a_path_git_source](normalize_a_path_git_source.md) - after the pedantic follow-ups: latent faults of a path `.git` source that SYN-49 now names, none observed in use
- [drop_the_harness_alias](drop_the_harness_alias.md) - after the rename: only once every consumer runs `cardinal`
- [drop_the_old_names_of_the_consumer_files](drop_the_old_names_of_the_consumer_files.md) - after the alias: only once every consumer is migrated to the new names
- [measure_the_effect_of_the_language_hint](measure_the_effect_of_the_language_hint.md) - last: needs sessions run with the hook before it can measure anything
