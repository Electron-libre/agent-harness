# Exit 3 when the launcher cannot parse its arguments

# Goal
A hook command the installed `harness` does not know (a new subcommand, a new global flag) exits 2 through the clap
parser, which Claude Code reads as the refusal of the hook: every hook then blocks and the agent is locked out. BIN-15
and BIN-16 probe the hooks before they take effect, but only through `check` and `update`; any other way a hook
command reaches a project still locks it out. A launcher that exits 3 on a parse error, as `run` already does when it
cannot get a tool (BIN-14), would never block the agent, whatever the command.

# Findings
* The `high` review of `install_the_new_harness_before_merging_a_hook_change` (2026-10-27) named it as the cause the
  probes work around: `main` uses `Cli::parse`, which exits 2 on an unknown subcommand.
* Paths the probes do not cover: `sync` of a local source, a hook of `settings.local.json`, a hook written by the
  project, a hook command that starts with an environment variable or an absolute path.
* Only launchers installed after the change benefit: an older one still exits 2.

# Approach
1. `main` parses with `Cli::try_parse`; on a parse error, it prints clap's message and exits 3, except for
   `--help` and `--version`, which keep exit 0.
2. Decide whether a mistyped command typed by a person (not a hook) may exit 3 rather than 2, or limit it to the
   commands a hook runs (`run`, the hidden `tool`).

# Specs
* `specs/tool-binaries.md`: a new requirement next to BIN-14.

# Resources
* `crates/harness/src/main.rs` (`main`, `CANNOT_RUN`), `specs/tool-binaries.md` (BIN-14, BIN-15, BIN-16)
* `issues/done/install_the_new_harness_before_merging_a_hook_change.md`

# Hints
* Claude Code blocks on exit 2 only; any other non-zero code is a non-blocking error shown to the user.
