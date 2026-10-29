# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [drop_the_old_names_of_the_consumer_files](drop_the_old_names_of_the_consumer_files.md) - after the alias: only once every consumer is migrated to the new names
- [report_what_a_format_migration_leaves_to_the_project](report_what_a_format_migration_leaves_to_the_project.md) - after the old names are dropped: no consumer is left to migrate, it serves the next format change
- [measure_the_effect_of_the_language_hint](measure_the_effect_of_the_language_hint.md) - last: needs sessions run with the hook before it can measure anything
- [fail_the_ci_installer_when_the_configuration_is_missing](fail_the_ci_installer_when_the_configuration_is_missing.md) - last: a pre-existing gap of the CI install, no consumer lacks its configuration today
