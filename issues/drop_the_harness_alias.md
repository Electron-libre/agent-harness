# Drop the harness alias

# Goal
The binary was renamed `cardinal` with a `harness` alias and fallbacks so the consumers synced before the rename kept
running. Once every consumer runs `cardinal`, drop what only served the transition, so the code and the release name
one binary.

# Findings
* The crate installs a second bin, `harness` (`crates/cardinal/src/bin/harness.rs`, BIN-18).
* `release.yml` publishes each commit twice, as `cardinal-<commit>` and `harness-<commit>`;
  `install_cardinal.sh` links `harness` to the installed `cardinal`.
* The git gate runs `cardinal`, else `harness` (BIN-9); `check` and `update` judge the `harness` hooks and count a
  missing `cardinal` as refused when `harness` is there (BIN-15, BIN-16); `gate::REVIEW_TOOLS` recognises the review
  hook under both names (SYN-22, SYN-23).

# Approach
1. Measure first: list the consumers whose lock still records a `harness run` hook or whose machine has no
   `cardinal`; drop nothing while one is left.
2. Remove the `harness` bin, the `harness-<commit>` publication and link, and the `harness` fallbacks of the gate,
   `check`, `update` and `REVIEW_TOOLS`, with their tests, and the recipe `check-harness` of `cardinal.just`
   (renamed `check-cardinal`).
3. Keep reading the `harness-<commit>` releases of the commits from before the rename (BIN-3) and the `harness run`
   refusals of old transcripts (EVA-5): old pins and old sessions stay readable.

# Specs
* `specs/tool-binaries.md`: BIN-18 withdrawn, BIN-9, BIN-15, BIN-16 amended; `specs/sync.md`: SYN-22, SYN-23
  amended.

# Resources
* `issues/done/rename_the_binary_to_cardinal.md` (the transition and its decisions)
* `crates/cardinal/src/gate.rs`, `launcher.rs`, `src/bin/harness.rs`, `.github/workflows/release.yml`,
  `files/.github/install_cardinal.sh`

# Hints
* The cache moved to `<cache>/cardinal`: the old `<cache>/harness` (clones, binaries) can be deleted then.
* A gate already installed calls `harness` when `cardinal` is missing: `sync` reinstalls it with the new script.
