# Allow cardinal run in the synced projects

# Goal
The managed skills tell the agent to run `cardinal run <tool>`, but the rule that allows it without a prompt,
`Bash(cardinal run *)`, is only in the seeded settings, copied once. A project seeded before the rename allows
`Bash(harness run *)` only, so each `cardinal run` of a skill asks for a permission (denied in a non-interactive run).
Make the synced projects allow `cardinal run` without the user editing their settings.

# Findings
* `seed/.claude/settings.json` holds `Bash(cardinal run *)` since the binary rename; the managed entries of
  `entries/.claude/settings.json` hold deny rules and hooks, no allow rule (SYN-29).
* Found by the `high` review of `rename_the_binary_to_cardinal`.

# Approach
1. Let `entries/.claude/settings.json` manage allow rules as it manages deny rules, and declare
   `Bash(cardinal run *)` there.
2. Or have `check` and `sync` warn when the settings allow `Bash(harness run *)` and not `Bash(cardinal run *)`,
   as SYN-24 warns of a missing deny rule.

# Specs
* `specs/sync.md` (SYN-29 to SYN-36 for managed entries, or a warning next to SYN-24).

# Resources
* `crates/cardinal/src/entries.rs`, `crates/cardinal/src/settings.rs`, `entries/.claude/settings.json`,
  `seed/.claude/settings.json`, ADR 0007

# Hints
* The hooks themselves need no allow rule: only the commands the agent runs through Bash.
