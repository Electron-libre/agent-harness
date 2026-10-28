# List the user-level instruction files in the instruction audit

# Goal
`harness run audit-sources` lists the instruction files of each project's working directories (AUD-1) but not the
user-level ones every session loads too: `CLAUDE.md` and the `rules/**/*.md` files of the Claude Code configuration
directory of the home directory. A rule the user keeps there is invisible to `audit-instructions`, which may propose
it again. List them once, apart from the projects.

# Findings
* 2026-10-28, step 2 of `issues/done/list_the_always_on_rules_in_the_instruction_audit.md`: left out of scope (decision
  `1 a`), AUD-1 lists them among its non-goals.

# Approach
1. Print them once before the projects (`instructions: <path>`), resolved with `memory.rs` from the configuration
   directory (`CLAUDE_CONFIG_DIR` when set), imports included.
2. Or name them in the "Collect" step of `audit-instructions` only, the skill reading them itself.

# Specs
* `specs/instruction-audit.md`: a new requirement, the non-goal withdrawn.

# Resources
* `crates/harness/src/tools/audit_sources.rs`, `crates/harness/src/memory.rs`
* `files/.claude/skills/audit-instructions/SKILL.md` (step "Collect")
