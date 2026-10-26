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
