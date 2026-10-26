# Make the checks quiet and cheap

# Goal
Everything a check prints is read by the agent and billed as tokens, every turn it runs. Have the checks say
nothing on success and little on failure, and run the Stop hook only when there is something new to check.

# Findings
* The Stop hook `clippy_gate.rs` runs `cargo clippy --workspace --all-targets` at every end of turn while any `.rs`
  file is dirty in `git status`, even when nothing changed since its last run, and sends the whole stderr to the agent.
* `just check` and `just test-harness` print the full output of the tests on success.
* The amount of tokens these outputs take per issue is not measured yet.

# Approach
1. Measure: the output size of `just check`, `just test-harness` and of a failing clippy gate on this repository.
2. Quiet recipes: quiet flags for the tests (`cargo test -q`), and test scripts that print only failures.
3. Clippy gate: send only the first errors (for example 40 lines) and a count; skip the run when no Rust file changed
   since the last passing run (a stamp in the git directory).
4. Document the model setup that saves the most (for example a planning model for steps 1 to 3 and a cheaper one for
   step 4 and for the review sub-agent) in `docs/architecture.md`, without imposing it in the shared settings.

# Resources
* `files/.claude/hooks/clippy_gate.rs`, `files/.claude/hooks/test_hooks.rs`
* `files/harness.just`, `justfile`
* `issues/done/cheapen_the_commit_gate_hooks.md` (earlier latency work on the hooks)

# Hints
* The gate must still let go when `stop_hook_active` is set, to avoid an endless loop.
* A truncated clippy output must keep the file and line of each error kept.

# Specs
* `specs/clippy-gate.md` (CLG)

# Tracking
1. [x] Clippy gate: short format, at most 40 diagnostics, stamp of the last passing run (CLG-1 to CLG-5).
2. [ ] Quiet recipes: `test-harness` (`files/harness.just`), `check`, `clippy`, `test` (`justfile`) print nothing on success, only the failures otherwise.
3. [ ] Docs: the cheapest model setup and the measures before/after in `docs/architecture.md`.

## Decisions
* Mode: autonomous.
* Base commit: 57303d89ed439426d29d25a8e8e8614482d131dc.
* Measures before (this repository): `just check` 86 lines / 5611 bytes, `just test-harness` 267 lines / 12706 bytes on success; a failing clippy gate with 3 errors 35 lines / 1854 bytes (the short format: 3 lines).
* The gate uses `--message-format=short` (one line per diagnostic, with the file, line and help) rather than cutting the human output.
* `test-harness` is quieted in the recipe, which hides the output of a passing script and shows only its non-`ok` lines when it fails; the test scripts are unchanged.
* A failing run writes no stamp: the next end of turn runs clippy again even when nothing changed.

## Deliverable 1
* done: the clippy gate reports one short line per diagnostic (at most 40, then a count), skips clippy while the Rust state of its last passing run is unchanged; spec `specs/clippy-gate.md` accepted.
* checks: `just check` ok, `just test-harness` ok, specs traced (CLG-1 to CLG-5), docs `docs/architecture.md`.
* red: `FAIL - gate reports the file, line and message`, `... at most 40 diagnostics`, `... skips clippy while the Rust state is unchanged` (and 3 more).
* Code review (low): the stamp ignores `Cargo.toml`, `clippy.toml` and `[lints]`; not fixed, a non-goal of `specs/clippy-gate.md` (the next `.rs` edit or `just check` catches it).
* not verified: the gate in a live Claude Code session.
