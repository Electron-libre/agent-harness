# Report what a format migration leaves to the project

# Goal
When `update` migrates a project to a new format (as SYN-45 did, withdrawn by ADR 0016), it renames the managed names, but the project-owned files
still cite the old ones, and nothing tells the agent the migration exists before it runs `update`. In the two
migrations to `cardinal`, the agent of each consumer spent its session finding and fixing those leftovers by hand.
Make the next format migration discoverable and make `update` say, or fix, what it leaves behind.

# Findings
* Measured on 2026-10-29, the sessions that migrated the two consumers from `harness` to `cardinal` (Claude Code,
  Opus): each found that `update` migrates by reading ADR 0015 and `docs/usage.md`; one cloned the source into a
  temporary directory to read them. Nothing printed by `check` or `sync` on a project with `.claude/harness.toml`
  pointed at `cardinal update`.
* The `source` URL still named the repository before its rename; it was edited by hand in both (`sed` by the agent
  in one, by the user in the other). The old URL still resolves through the forge redirect.
* `update` rewrote the `import` line of the justfile (SYN-45) but not the `check: check-harness` dependency of the
  seeded `check` recipe, nor the header comments of `.claude/cardinal.toml`, still the old seed's text word for word
  (`the shared harness reads … harness sync`, `besides docs and harness`).
* About six `grep`/`sed` calls per consumer went to the old names left in project-owned files: `CLAUDE.md`,
  `README.md`, `docs/`, `.claude/update-docs-map.md` (`harness.toml`, `harness run`, `check-harness`,
  `cargo install … harness --locked`, `install_harness.sh`, `harness.yml`).

# Approach
1. `update`, after a migration, warns with the lines of the project-owned files (tracked by git, outside the
   managed files, `issues/done/` and `docs/adr/`) that still cite an old managed name, `path:line`, without
   rewriting them.
2. Extend the migration to the seeded text it can recognise exactly: the `check-harness` dependency of the
   justfile, the header comments of the configuration when they are byte-identical to the old seed.
3. `check` and `sync` on a project below the current format with a source of a newer one print a hint to run
   `cardinal update`.
4. Optionally, `update` rewrites a `source` URL the forge redirects (a renamed repository), or only reports it.

# Specs
* `specs/sync.md`: new requirements for the migration, the leftover report and the migration hint.

# Resources
* ADR 0014, ADR 0015, ADR 0016
* `crates/cardinal/src/lib.rs` before `faea050` (`Migration`, `imports_renamed`), the migration it dropped
* `issues/done/rename_the_consumer_files_to_cardinal.md`

# Hints
* No consumer is left to migrate to `cardinal`: the value is for the next change of `FORMAT`, so the old names to
  look for come from the format change, not hard-coded `harness` strings.
* The format 2 migration code is gone (`drop_the_old_names_of_the_consumer_files`): the next migration brings its
  own, and this issue builds on it.
