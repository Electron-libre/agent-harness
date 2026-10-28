# Relink the relative links to a closed issue

# Goal
`just close-issue issues/foo.md` rewrites `issues/foo.md` to `issues/done/foo.md` in the text files of the repo, but
leaves the relative Markdown links of `docs/` (`[x](../issues/foo.md)`, `../../issues/foo.md` from `docs/adr/`)
pointing at the old path. After a close, every link to the issue, relative or not, must resolve to
`issues/done/foo.md`.

# Findings
* Reproduced on 2026-10-28 in a temporary git repo with `harness run close-issue issues/foo.md`: `` `issues/foo.md` ``
  in `docs/x.md` and `docs/adr/y.md` became `issues/done/foo.md`; `[a](../issues/foo.md)` and
  `[b](../../issues/foo.md)` stayed unchanged.
* Cause in `relink_text` (`crates/harness/src/tools/close_issue.rs`): `issues/` is rewritten only when the character
  before it is whitespace or one of `` `"'([<{=,: ``; a `/` (as in `../issues/`) is not a boundary, on purpose so
  that `docs/issues/` is not touched.
* The match is textual: it does not resolve a link against the directory of the file that holds it, so it cannot
  tell `../issues/foo.md` from `docs/issues/foo.md`.
* No spec covers the relinking of `close-issue` (only PAR-8 and PAR-14 of `specs/parallel-issues.md` name it).
* `--relink` uses the same `relink_text`, so it does not repair the old relative links either.

# Approach
1. Resolve each candidate link against the directory of the file that holds it (`../issues/foo.md` from `docs/`
   is `<root>/issues/foo.md`), and rewrite it when it designates the closed issue file, keeping its relative form
   (`../issues/done/foo.md`).
2. Keep the textual match for the paths written from the root (`issues/foo.md` in code spans and prose), as today.
3. Write the requirements of the relinking in a spec (amend `specs/parallel-issues.md` or a new
   `specs/close-issue.md`), with a test per form: root path, `../`, `../../`, and `docs/issues/foo.md` left alone.

# Specs
* The relinking of `close-issue` (root paths and relative links): no spec yet, to write at `issue-workflow` step 1.

# Resources
* `crates/harness/src/tools/close_issue.rs` (`relink_text`, `relink`, `--relink`)
* `specs/parallel-issues.md` (PAR-8, PAR-14), `crates/harness/src/tools/repo_files.rs`

# Hints
* A link between two issues (`[x](bar.md)` in `issues/foo.md`) also breaks once one of them moves to `done/`: in
  scope or not is a decision of step 1.
* Run `--relink` on this repository once fixed, to repair the relative links already dangling (none found in
  `docs/` on 2026-10-28).
