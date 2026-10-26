# Warn of the settings at sync

# Goal
`harness check` warns when the seeded `.claude/settings.json` lacks a deny rule of the review waiver, runs a hook
the project no longer has, or leaves the attribution on (SYN-24 to SYN-26). `harness sync` says nothing, yet it is
the sync that deletes a hook the source dropped (SYN-5): from then on the settings run a missing script and every
`Bash` call shows a hook error until someone runs `check`. Make the sync tell it at once.

# Findings
* Seen when `block_ai_attribution.rs` left `files/`: `sync` reported `removed: .claude/hooks/block_ai_attribution.rs`
  and nothing about the settings that still declare it; only `check` printed the missing hook.
* The warnings come from `settings::warnings` in `crates/harness/src/settings.rs`, called by `check_with` only.
* A hook whose script is missing exits 1, a non-blocking error in Claude Code: noise, not a refusal.

# Approach
1. After a sync (not a dry run), print the warnings of `settings::warnings` to stderr, without failing.
2. Or print them only when the sync removed a hook, or when the settings name a hook the sync just removed.
3. Under `--dry-run`, say which declared hooks the sync would leave missing.

# Specs
* `specs/sync.md`: amends SYN-25 and SYN-26 (and possibly SYN-24) so that `sync` prints them too.

# Resources
* `crates/harness/src/settings.rs`, `crates/harness/src/lib.rs` (`sync`, `check_with`), `crates/harness/src/main.rs`
* `specs/sync.md` (SYN-5, SYN-24 to SYN-26)
* `issues/done/drop_redundant_guards_and_prose_tests.md`

# Hints
* `sync` is also run by `update`: the warnings should come out once.
