# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.
- [update_the_harness_to_a_revision](update_the_harness_to_a_revision.md) - removes the by-hand `rev` edits that the adoption introduces; comes right after it
- [match_config_keys_exactly](match_config_keys_exactly.md) - wrong scopes or dirs silently used by synced scripts; small fix
- [fail_loudly_when_harness_toml_is_unreadable](fail_loudly_when_harness_toml_is_unreadable.md) - same script, same family of silent failures
- [share_the_harness_toml_reader](share_the_harness_toml_reader.md) - removes the triplicated parser; best done with the two above
