# Issue queue

Open issues in the order to work on them: the first is the next one. Read by `just next-issue`;
`just close-issue` removes a line, `skill:create-issue` inserts one at its rank, `skill:prioritize-issues`
re-ranks the whole list. Line format: `- [<file stem>](<file stem>.md) - <why here>`.

- [check_the_red_run_of_traced_tests](check_the_red_run_of_traced_tests.md) - small and independent; makes the spec traceability prove more than a citation
- [prove_the_specs_with_black_box_tests](prove_the_specs_with_black_box_tests.md) - after the red-run evidence, the other half of making a citation a proof; sets the rule the next two apply
- [prove_the_sync_specs_through_the_binary](prove_the_sync_specs_through_the_binary.md) - after the rule and its report; moves the SYN proofs of init, sync, update, check and the settings entries
- [prove_the_run_and_gate_specs_through_the_binary](prove_the_run_and_gate_specs_through_the_binary.md) - after the sync proofs, same method; moves the run, launcher, install gate and retention proofs
- [make_the_architecture_doc_skimmable](make_the_architecture_doc_skimmable.md) - after the spec proofs, which move edge cases into the specs; unblocked by the binaries
- [bootstrap_specs_for_an_existing_capability](bootstrap_specs_for_an_existing_capability.md) - widest scope, benefits from the spec clarity checks and the proof rule first
