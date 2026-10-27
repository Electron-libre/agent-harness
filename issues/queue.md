# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [prove_the_run_and_gate_specs_through_the_binary](prove_the_run_and_gate_specs_through_the_binary.md) - after the sync proofs, same method; moves the run, launcher and retention proofs
- [prove_the_gate_specs_through_the_binary](prove_the_gate_specs_through_the_binary.md) - after the run proofs, reuses their fake programs; moves the install gate and settings warnings proofs
- [make_the_architecture_doc_skimmable](make_the_architecture_doc_skimmable.md) - after the spec proofs, which move edge cases into the specs; unblocked by the binaries
- [bootstrap_specs_for_an_existing_capability](bootstrap_specs_for_an_existing_capability.md) - widest scope, benefits from the spec clarity checks and the proof rule first
- [weigh_the_form_and_the_evidence_of_harness_proposals](weigh_the_form_and_the_evidence_of_harness_proposals.md) - low priority, skill text only; sets the evidence rules the verdict issue applies
- [budget_the_fixed_context_of_a_project](budget_the_fixed_context_of_a_project.md) - low priority, independent; makes the cost of the always-on text visible before it grows
- [record_a_verdict_for_each_harness_change](record_a_verdict_for_each_harness_change.md) - low priority, last: applies the evidence rules of the first and reads the metrics of the report
