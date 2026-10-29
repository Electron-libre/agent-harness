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

# Tracking

Plan: validated

## Understanding
A project declares `lint_command`/`lint_files` and `format_command`/`format_files` in the `[project]` table of its own
`.claude/cardinal.toml`; absent, the hooks keep the clippy and `cargo fmt` behaviour. Out of scope: several linters
per project (one command, the project chains them), parsing a non-clippy output (cut to 40 lines).

## Plan
1. [x] Rename the hooks: `clippy-gate` → `lint-gate`, `format-rust` → `format-code` (tools, `entries/.claude/settings.json`,
   format 4 with `migrations/4.toml` naming the old names, ADR 0014/0017; `specs/clippy-gate.md` → `specs/lint-gate.md`,
   ids kept; PAR-12, docs), no behaviour change; verified by `rust_hooks` and the migration/format tests; makes true none
   (rename). ~350 lines.
2. [ ] Generic lint gate: `lint_command`, `lint_files`, the stamp over the declared patterns and the command, output cut
   to 40 lines, clippy kept as default; verified by tests with a fake linter; makes true CLG-1, CLG-4, CLG-5, CLG-7,
   CLG-8. ~400 lines.
3. [ ] Generic format hook: `format_command`, `format_files`, file appended, run from the work tree root, silent;
   verified by tests with a fake formatter; makes true FMT-1 to FMT-4. ~250 lines.
4. [ ] Reword `issue-workflow` (step 3.3, 3.5) and its `details.md` around "the lint gate" and `just check`; document
   the keys in `seed/.claude/cardinal.toml` and `docs/architecture.md`; verified by `just check` (skills tests);
   makes true none (docs). ~60 lines.

## Decisions
- Mode: autonomous
- Base commit: d9753896d8ac2e7f6ef9d82673568be7198c1298
- Hook names: `lint-gate` and `format-code`, the format raised to 4 with `migrations/4.toml` (1 a).
- The format command gets the edited file as its last argument (2 a).

## Deliverable 1
- done: `lint-gate` and `format-code` replace `clippy-gate` and `format-rust` (tools, settings entries, docs, spec
  `specs/lint-gate.md`); format 4, `migrations/4.toml` names the old names.
- check | result: tests ok, lint ok, specs ok, docs ok (architecture, PAR-12), code review: see below.
- red: none (rename, no new behaviour; `rust_hooks` and `migration_cli` adapted).
- Code review: no findings
- not verified: a consumer on the format 3 updated to this revision (the migration warnings were seen here only).
- decisions needed: none.
