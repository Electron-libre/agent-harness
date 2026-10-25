# Trace the specs cited by managed tests strictly in the harness repository

# Goal
Keep the strict `// spec:` check for the tests under `.claude/` in this repository, while consumers keep the
lenient rule. A mistyped prefix or a spec dropped while a managed test still cites it must fail here.

# Findings
* Since 9a916f4, `check_specs.rs` skips a citation under `.claude/` whose spec prefix is not defined in the
  project, so that a project without `specs/parallel-issues.md` passes `just check-specs`.
* The same rule applies in this repository: `// spec: SYM-3` instead of `SYN-3` is accepted, and so is a
  managed test citing a spec that was deleted or renamed here. Found by the review of that commit, not fixed.

# Approach
1. Add an option to the checker (for example `--strict`) that disables the exemption, and call it from the
   `check-specs` recipe of this repository only; the managed `harness.just` keeps the lenient call.
2. Alternative: a recipe of the root `justfile` that runs the checker strictly next to `just check`.
3. Either way, add test cases: strict mode flags a missing prefix under `.claude/`, default mode still accepts it.

# Specs
* No requirement to add: the rule is documented in `specs/README.md` (copy of `files/specs/README.md`); amend
  it to mention the strict mode if it ships in a managed file.

# Resources
* `files/.claude/specs/check_specs.rs`, `files/.claude/specs/test_check_specs.rs`, `files/specs/README.md`,
  `files/harness.just`, `justfile`

# Hints
* Consumers must not run the strict mode: they do not have the specs of the managed scripts.
