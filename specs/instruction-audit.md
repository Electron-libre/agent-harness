---
id: AUD
status: accepted
adrs: [0006]
---

# Sources of the instruction audit

## Intent

`harness run audit-sources` lists, per project of the transcripts directory, what the user told the agent over time
(sessions, typed messages, memory files, instruction files), so that `audit-instructions` proposes only the skills,
hooks and rules that do not exist yet.

## Requirements

- **AUD-1** (deterministic) Without `--messages`, `audit-sources` must print, for each working directory of a project's sessions, one line `  instructions: <cwd>/<path>` per instruction file of that directory, sorted by `<path>`, each file once however many paths name it (a symbolic link printed as the file it points to, by its absolute path when that file is outside the directory): the memory files a session loads at launch as REP-2 defines them (imports included), plus `AGENTS.md`, `.claude/AGENTS.md` and the `.claude/rules/**/*.md` files whose front matter has a `paths` key, when they exist.

## Scenarios

- (AUD-1) Given a directory with `CLAUDE.md`, `AGENTS.md`, a rule `.claude/rules/a.md` and a rule `.claude/rules/b.md` scoped by `paths`, when the summary is printed, then it lists the four files.
- (AUD-1) Given a `CLAUDE.md` that imports `@docs/x.md` and `@~/notes.md`, when the summary is printed, then it lists `docs/x.md` and not `~/notes.md`.
- (AUD-1) Given a `CLAUDE.md` that is a symbolic link to `AGENTS.md`, when the summary is printed, then it lists `AGENTS.md` once.
- (AUD-1) Given a rule that is a symbolic link to `/shared/rule.md`, when the summary is printed, then it lists `/shared/rule.md`.

## Non-goals

- The user-level files (`~/.claude/CLAUDE.md`, `~/.claude/rules/`): they are not a project's.
- The content of the files: the skill reads them.
