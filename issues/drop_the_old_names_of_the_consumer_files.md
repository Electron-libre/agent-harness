# Drop the old names of the consumer files

# Goal
Once every consumer has migrated to `.claude/cardinal.toml`, `.claude/cardinal.lock` and `cardinal.just`, stop
reading and writing `.claude/harness.toml` (its `[harness]` table), `.claude/harness.lock` and `harness.just`, so the
tool loses the fallback and the migration code it keeps for them.

# Findings
* The old names live in `crates/cardinal/src/lib.rs` (`OLD_CONFIG`, `OLD_LOCK`, `Names` and its `OLD`, `Migration`,
  `table_renamed`, `imports_renamed`), `run.rs` (`project_root`), `gate.rs` (the `pre-commit` script),
  `files/.github/install_cardinal.sh` and `tools/transcripts.rs` (`CONFIGS`, `LOCKS`).
* They are written for a source below the format 3 (SYN-44): dropping them means `cardinal` stops reading the
  format 2, which ADR 0014 says it reads with every lower one.

# Approach
1. Check that every consumer is pinned to a revision of the format 3 or later, and migrated.
2. Decide whether this `cardinal` stops syncing sources below the format 3 (a new ADR superseding part of ADR 0014
   and ADR 0015) or keeps writing the old names for them only.
3. Keep the old names in the history readers (`transcripts.rs`): past commits still hold them.
4. Withdraw or amend SYN-44 to SYN-47 and the specs that cite them.

# Specs
* `specs/sync.md` (SYN-44, SYN-45, SYN-46, SYN-47), `specs/tool-binaries.md` (BIN-1, BIN-14).

# Resources
* ADR 0014, ADR 0015, `docs/usage.md` (the migration paragraph)
* `issues/done/rename_the_consumer_files_to_cardinal.md`

# Hints
* After `drop_the_harness_alias`: both drop what a consumer of before the rename still needs.
