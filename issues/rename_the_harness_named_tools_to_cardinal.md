# Rename the harness-named tools to cardinal

# Goal
Rename what still carries "harness" as the product name once the binary and files are renamed: the skill
`evaluate-harness`, the tools `harness-report` and `harness-changes`, the spec `harness-verdicts`, the commit scope
`harness`, the CI workflow `harness.yml` and the recipe `check-harness`.

# Findings
* `evaluate-harness` appears in 25 lines of 14 files, `harness-report` in 28 lines of 16 files, `harness-changes`
  in 21 lines of 12 files, `harness-verdicts` in 5 lines.
* `AGENTS.md` makes `harness` a scope that always exists (`.claude/`, `justfile`, CI); `check-commits` enforces it,
  and `harness-changes` may read past commits by it.
* `CLAUDE.md` defines the `Expected-effect` footer against the metrics of `harness-report`.

# Approach
1. Decide per name: rename to `cardinal-…`, or keep the generic noun where it means the concept (e.g.
   `evaluate-harness` evaluates an agent harness and may stay).
2. Keep reading the old commit scope in the history tools, so past commits still count.
3. `git mv` in a deliverable of its own, then the references, then `just sync`.

# Specs
* `specs/harness-report.md`, `specs/harness-verdicts.md` and the specs citing these tools; renamed or amended at
  `issue-workflow` step 1.

# Resources
* `files/.claude/skills/evaluate-harness/`, `crates/cardinal/src/tools/harness_report.rs`, `harness_changes.rs`
* `AGENTS.md`, `files/.github/workflows/harness.yml`, `justfile`

# Hints
* Also the product prose that `name_the_product_cardinal` left out of its scope: the specs (e.g. `specs/sync.md`
  "keeps the shared harness", `specs/harness-verdicts.md` "harness source") and the doc comments of the crate
  (`lib.rs`, `tools/harness_changes.rs`).
* After `rename_the_consumer_files_to_cardinal`.
* Renaming a skill changes the fixed context of every session and the skill names counted by `harness-report`:
  past sessions cite the old name.
