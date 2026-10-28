# Read the tests of a project whatever its layout

# Goal
`check-specs` and `spec-coverage` read the tests of `crates/` and `.claude/` only, the layout of this repository. A
consumer whose crate sits at its root (`src/`, `tests/`) has its `// spec:` citations ignored, so an `accepted` spec of
it looks untraced, and `spec-coverage` reports no test file, which hides every capability worth a spec. Read the
tests of any layout, or say which directories hold them.

# Findings (high review of `bootstrap_specs_for_an_existing_capability`, 2026-10-28)
* `check_specs::test_files` lists the `.rs` files under `<root>/crates` and `<root>/.claude` only, through
  `repo_files` (ADR 0009).
* `spec-coverage` reuses it: in a root holding `src/lib.rs` and `tests/cli.rs` with `#[test]`s, it prints
  `no test file under crates/ or .claude/` (COV-9 states it, a non-goal of `specs/spec-coverage.md` keeps the limit).
* The managed tests under `.claude/` must stay read whatever the layout.

# Approach
1. Reproduce with a test through the binary: a root with `src/` and `tests/` at its top, citations in both.
2. Candidates (can be combined):
   1. Read every `.rs` file git lists, the nested worktrees already left out by `repo_files`.
   2. A `test_dirs` key of `.claude/harness.toml`, defaulting to `crates` and `.claude`.
3. Keep the split between the tests under a `src` directory and the others (TRC) right for a root `src/`.

# Specs
* `specs/spec-tracing.md` (the files read, TRC-5), `specs/spec-coverage.md` (COV-1, COV-9 and the non-goal).

# Resources
* `crates/harness/src/tools/check_specs.rs` (`test_files`), `crates/harness/src/tools/spec_coverage.rs`,
  `crates/harness/src/tools/repo_files.rs`, `docs/adr/0009-scanners-list-the-repository-files-through-git.md`,
  `seed/.claude/harness.toml`

# Hints
* Reading every `.rs` file also reads the build scripts and the examples: a `// spec:` there must still sit above a
  test `fn`.
