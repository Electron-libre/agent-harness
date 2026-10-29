# Run the lint and format of the project stack

# Goal
The two code hooks of Cardinal are Rust-only: `clippy-gate` (Stop) blocks a turn while `cargo clippy` warns, and
`format-rust` (PostToolUse) runs `cargo fmt` after an edit of a `.rs` file. In a project of another stack they do
nothing, so the agent gets neither the lint gate nor the formatting. Let a project declare its lint and format
commands, Rust staying the default, and remove the Rust wording from the managed skills.

# Findings
* CLG-1 (`specs/clippy-gate.md`): the gate exits 0 without running anything when the project has no `Cargo.toml`.
  CLG-2 to CLG-5 parse clippy's output and stamp the state of the `.rs` files.
* `crates/cardinal/src/tools/format_rust.rs` acts only on a `file_path` with the `rs` extension and runs
  `cargo fmt --all`; it has no spec.
* `files/.claude/skills/issue-workflow/SKILL.md` names "the clippy gate hook" (step 4) and `just check` as "fmt,
  clippy, tests, specs" (step 5); its `details.md` names `clippy-gate` and `format-rust`. The seeded `justfile`
  already asks the project to put its own format, lint and tests in `check`.
* Every consumer measured on 2026-10-29 is a Rust project.

# Approach
1. Add keys to `.claude/cardinal.toml`: the lint command and the file patterns that make it run, the format command
   and the patterns it applies to; their absence keeps the Rust behaviour.
2. Turn `clippy-gate` into a generic lint gate: same stamp (CLG-4, CLG-5) over the declared patterns, the output cut
   to a number of lines (CLG-3) when it cannot be parsed per diagnostic; keep clippy's parser for Rust.
3. Turn `format-rust` into a generic format hook, the hook name in `entries/.claude/settings.json` changing or not
   (a renamed managed entry declares a migration, ADR 0017).
4. Reword `issue-workflow` and its `details.md` around "the lint gate" and `just check`, without naming a stack.

# Specs
* `specs/clippy-gate.md` (amended or replaced by a lint gate spec); a new spec of the format hook.

# Resources
* `crates/cardinal/src/tools/clippy_gate.rs`, `format_rust.rs`, `entries/.claude/settings.json`.
* `docs/architecture.md` (table of the hooks), `docs/adr/0005-the-gates-catch-mistakes-not-bypasses.md`.

# Hints
* Everything a hook prints is read by the agent and billed: keep the output short for any linter.
* A project command run from a hook must come from the project's own file, never from the shared one.
