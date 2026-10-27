# Propagate new hook declarations on sync

# Goal
A hook added to `files/.claude/hooks/` reaches a project that already adopted the harness only when its declaration
is added by hand to the project's `.claude/settings.json`, which is seeded once. Make `harness sync` (or `update`)
able to add the managed hook declarations itself, without taking ownership of the rest of the settings.

# Findings
* `docs/architecture.md`, "Hooks and their declaration": `settings.json` is seeded, so a new hook needs a manual
  edit in each project; `harness check` and `sync` only warn (issue `warn_of_the_settings_at_sync`, done).
* The same applies to the `permissions.deny` rules of the review waiver (`DENY_RULES` of
  `crates/harness/src/settings.rs`): a project synced before they existed lacks them.
* Switching the hooks from `rust-script` to prebuilt binaries (issue `ship_the_harness_scripts_as_prebuilt_binaries`)
  would change every hook command, so it needs this first.

# Approach
1. Measure: list what differs today between the seeded settings and the settings of the consumer projects
   (hooks, deny rules, attribution), to size the need.
2. A managed fragment: the harness owns a set of entries (hook declarations marked as its own, deny rules) and
   `sync` adds, updates or removes only those, leaving every other key as the project wrote it.
3. Or Claude Code's own layering: put the managed declarations in a separate settings file Claude Code reads
   (check which files it merges) and keep the project's file untouched.

# Specs
* `specs/sync.md`: new requirements for the managed settings entries.

# Resources
* `crates/harness/src/settings.rs`, `seed/.claude/settings.json`, `files/.claude/test_settings.rs`
* `docs/adr/0001-shared-harness-synced-by-a-rust-tool.md` (no merge of managed files)

# Hints
* ADR 0001 rejected merging managed files; a JSON entry merge is a different decision and needs its own ADR.
* JSON written back must keep the project's key order and formatting, or every sync makes a diff.
