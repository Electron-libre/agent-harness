# Trace the specs in the tests of any language

# Goal
`check-specs` reads the `// spec: ID` citations only in `.rs` files, so in a project whose tests are in Python,
TypeScript, Go or shell, every requirement reads as untraced and the traceability gate of `just check` cannot be
met. Read the citations in the test files of the languages a project declares, with the comment syntax of each, so
that specs, traceability and the `traced` columns of `delivered-quality` work on any stack.

# Findings
* TRC-9 (`specs/spec-tracing.md`): the checker reads "the citations of every `.rs` file under the root";
  `crates/cardinal/src/tools/check_specs.rs` lists the files with `repo_files(root, &["*.rs"])` and filters on the
  `rs` extension, then keeps the lines whose trimmed text starts with `// spec:`.
* TRC-9 also skips a `target` directory beside a `Cargo.toml`: the build output of Rust only; other stacks have
  theirs (`node_modules` is skipped, `dist`, `build`, `.venv`, `__pycache__` are not).
* QUA-11 (`specs/delivered-quality.md`) counts `traced` from the added lines starting with `// spec:` in the files
  of an issue, whatever their extension: a `#` or `--` comment is never read.
* Every consumer measured on 2026-10-29 is a Rust project; no other stack has been tried.

# Approach
1. Declare the test languages in `.claude/cardinal.toml` (e.g. `spec_files = ["*.py", "*.ts"]`), the default staying
   `*.rs` so that a Rust project changes nothing.
2. Accept the citation after any common line-comment marker (`//`, `#`, `--`, `;`), or let the project give the
   marker per file pattern; the id list and the `(internal: <reason>)` of TRC-6 to TRC-8 stay the same.
3. Generalize the boundary rule of TRC-1 to TRC-5 (a test under `src` is internal) or make the internal directories
   a value, since other stacks keep their tests elsewhere (`tests/`, `__tests__/`, `*_test.go` beside the code).
4. Read the citations of QUA-11 with the same function, and check whether `spec-coverage` lists the test files the
   same way.

# Specs
* `specs/spec-tracing.md` (TRC-1, TRC-5, TRC-9, TRC-10), `specs/delivered-quality.md` (QUA-11),
  `specs/spec-coverage.md` if its file list changes; `files/specs/README.md` documents the citation form.

# Resources
* `crates/cardinal/src/tools/check_specs.rs`, `delivered_quality.rs`, `spec_coverage.rs`, `repo_files.rs`.
* `docs/architecture.md` ("What is shared, and how": project values go through `.claude/cardinal.toml`).

# Hints
* A new key read by an older `cardinal` is ignored, not misread; check whether it still needs `cardinal-format`
  (ADR 0014).
