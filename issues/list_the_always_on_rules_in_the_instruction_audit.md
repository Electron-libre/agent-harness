# List the always-on rules in the instruction audit

# Goal
`harness run audit-sources` lists as instruction files only `CLAUDE.md` and `AGENTS.md` of each project's working
directory. Since the rule "Reports to the user" moved to the managed `.claude/rules/reports-to-the-user.md`, the
audit no longer sees it, nor any rule a project keeps under `.claude/rules/`, so `audit-instructions` may propose a
rule or a skill that already exists. List every memory file a session loads at launch instead.

# Findings
* 2026-10-28, high review of `issues/done/load_the_shared_agents_md_in_every_session.md`: `INSTRUCTION_FILES` in
  `crates/harness/src/tools/audit_sources.rs` holds `CLAUDE.md` and `AGENTS.md` only.
* `crates/harness/src/memory.rs` already resolves the memory files loaded at launch (REP-2): `CLAUDE.md`,
  `.claude/CLAUDE.md`, the `.claude/rules/**/*.md` files without `paths`, their imports.

# Approach
1. List the instruction files from the resolution of `memory.rs` (it would need to return the paths, not only the
   bytes), plus `AGENTS.md` even when not loaded, since the audit reads what the user wrote.
2. Or add `.claude/CLAUDE.md` and the `.claude/rules/*.md` files to the fixed list, the scoped rules included.

# Resources
* `crates/harness/src/tools/audit_sources.rs`, `crates/harness/src/memory.rs`
* `files/.claude/skills/audit-instructions/SKILL.md` (step "Collect")
* `specs/harness-report.md` REP-2
