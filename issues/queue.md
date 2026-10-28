# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [load_the_shared_agents_md_in_every_session](load_the_shared_agents_md_in_every_session.md) - after the exact measure, which reports what it adds; before the evaluation issues: the shared rules are not loaded today
- [list_the_always_on_rules_in_the_instruction_audit](list_the_always_on_rules_in_the_instruction_audit.md) - next: the rule moved to `.claude/rules/` is invisible to the audit since the shared rules load from there; small, before the evaluation issues
- [record_a_verdict_for_each_harness_change](record_a_verdict_for_each_harness_change.md) - low priority, last: applies the evidence rules of the first and reads the metrics of the report
- [measure_the_effect_of_the_language_hint](measure_the_effect_of_the_language_hint.md) - last: needs sessions run with the hook before it can measure anything
