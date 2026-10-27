# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [align_the_tracking_section_end_of_qua_6](align_the_tracking_section_end_of_qua_6.md) - a one-line spec fix found by the red-run review; before the proof rule, which tests the specs as written
- [read_the_fences_of_the_issue_sections_as_the_specs_say](read_the_fences_of_the_issue_sections_as_the_specs_say.md) - small spec/code mismatch left by the QUA-6 review; before the proof rule, which tests the specs as written
- [prove_the_specs_with_black_box_tests](prove_the_specs_with_black_box_tests.md) - after the red-run evidence, the other half of making a citation a proof; sets the rule the next two apply
- [prove_the_sync_specs_through_the_binary](prove_the_sync_specs_through_the_binary.md) - after the rule and its report; moves the SYN proofs of init, sync, update, check and the settings entries
- [prove_the_run_and_gate_specs_through_the_binary](prove_the_run_and_gate_specs_through_the_binary.md) - after the sync proofs, same method; moves the run, launcher, install gate and retention proofs
- [make_the_architecture_doc_skimmable](make_the_architecture_doc_skimmable.md) - after the spec proofs, which move edge cases into the specs; unblocked by the binaries
- [bootstrap_specs_for_an_existing_capability](bootstrap_specs_for_an_existing_capability.md) - widest scope, benefits from the spec clarity checks and the proof rule first
- [weigh_the_form_and_the_evidence_of_harness_proposals](weigh_the_form_and_the_evidence_of_harness_proposals.md) - low priority, skill text only; sets the evidence rules the verdict issue applies
- [budget_the_fixed_context_of_a_project](budget_the_fixed_context_of_a_project.md) - low priority, independent; makes the cost of the always-on text visible before it grows
- [record_a_verdict_for_each_harness_change](record_a_verdict_for_each_harness_change.md) - low priority, last: applies the evidence rules of the first and reads the metrics of the report
