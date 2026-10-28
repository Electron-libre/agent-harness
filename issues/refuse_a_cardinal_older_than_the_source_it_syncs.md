# Refuse a cardinal older than the source it syncs

# Goal
An installed `cardinal` older than the source revision it syncs, or than the lock it reads, misreads the managed
content without saying so. `update` and `sync` must tell the user to install the matching `cardinal` instead of
syncing wrongly or failing with an obscure error.

# Findings
* Found by the `high` review of `allow_cardinal_run_in_the_synced_projects` (ADR 0013): the managed entries now hold
  `permissions.allow` rules.
* A binary from before ADR 0013 reads only `permissions.deny` and hooks in `entries/.claude/settings.json`: syncing a
  newer source drops the allow rule `Bash(cardinal run *)` silently, and the seed no longer carries it.
* The same binary reading a lock written by a newer one fails on an `entry {"allow": ...}` line with
  `not a settings entry` (exit 3), without saying to upgrade.
* The guard of `update` against an outdated binary (`launcher::refused`) only checks that the installed launcher
  accepts the hook subcommands; it does not see a change of the fragment or lock format.

# Approach
1. A format version: the source declares the minimum `cardinal` version it needs (in `entries/` or a source file),
   the lock records it; `sync`, `update` and `check` refuse with the install command when the running binary is
   older.
2. Or narrower: an unknown key in the fragment, or an unknown entry line in the lock, is an error that names the
   upgrade, instead of being ignored or reported as a malformed entry.

# Specs
* `specs/sync.md`: a new requirement next to SYN-29 and SYN-30.

# Resources
* `crates/cardinal/src/entries.rs` (`of_source`, `from_lock`), `crates/cardinal/src/launcher.rs` (`refused`),
  `crates/cardinal/src/lib.rs` (`update`)
* ADR 0007, ADR 0013, `issues/done/install_the_new_harness_before_merging_a_hook_change.md`

# Hints
* A binary already released cannot be changed: only the binaries from this issue on can refuse a newer source.
