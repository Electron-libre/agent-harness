# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [extract_session_metrics_from_the_transcripts](extract_session_metrics_from_the_transcripts.md) - the base every other evaluation issue reads
- [detect_rule_breaches_in_the_transcripts](detect_rule_breaches_in_the_transcripts.md) - after the extractor; cheap detectors that also feed audit-instructions
- [measure_the_delivered_quality_from_the_git_history](measure_the_delivered_quality_from_the_git_history.md) - after the extractor; mostly git and issue files
- [report_the_harness_evolution_by_revision](report_the_harness_evolution_by_revision.md) - assembles the three extractors into the report
- [replay_done_issues_to_compare_harness_revisions](replay_done_issues_to_compare_harness_revisions.md) - last: costly, only worth it once the passive report shows a trend to confirm
