# Exit 3 when `harness run` names an unknown tool

# Goal
`harness run` with no tool or a tool the pinned binary does not know exits 2 (BIN-7), which Claude Code reads as the
refusal of the hook: a hook command that names a mistyped or missing tool (written by the project, in
`settings.local.json`, or from a newer revision than the pinned one) blocks every command of the agent. An exit 3,
as BIN-14 and BIN-17 already give when the tool cannot be got or the launcher cannot parse its arguments, would tell
the user without locking the agent out.

# Findings
* `crates/harness/src/tools/mod.rs` (`run`, BIN-7) prints the list of the tools and returns 2 when the name is
  missing or unknown.
* Left out of `exit_3_when_the_launcher_cannot_parse_its_arguments` (2026-10-27): that issue only covers the parser of
  the launcher.
* BIN-15 and BIN-16 do not look at the words after `harness run`: the pinned binary judges them.

# Approach
1. Amend BIN-7 to exit 3 (keep the list of the tools on stderr) and its test.
2. Check the recipes and skills that may rely on exit 2 of an unknown tool.

# Specs
* `specs/tool-binaries.md`: amends BIN-7.

# Resources
* `crates/harness/src/tools/mod.rs`, `specs/tool-binaries.md` (BIN-7, BIN-14, BIN-17)
* `issues/exit_3_when_the_launcher_cannot_parse_its_arguments.md`

# Hints
* Claude Code blocks on exit 2 only; any other non-zero code is a non-blocking error shown to the user.
