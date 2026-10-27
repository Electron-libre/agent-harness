# Usage

The reference the [README](../README.md) leaves out. `harness help <command>` and `harness run` (the list of the
tools) say the same from the binary.

## Commands

```sh
harness init <source> [--rev <rev>]  # writes .claude/harness.toml; <source>: a directory (relative to the project) or a git URL
harness sync [--dry-run] [--force]
harness update [--rev <tag|branch|commit>] [--dry-run] [--force]
harness check
harness run <tool> [args]
```

| Command | What it does |
|---|---|
| `sync` | copies the managed files (`files/`), seeds the missing project-owned ones (`seed/`), adds the hooks and deny rules of the harness to `.claude/settings.json` (or removes those the source dropped), installs the git `pre-commit` review gate. A managed file edited locally is a conflict: fix it in the source, or `--force` to overwrite it. |
| `update` | git source only: resolves the revision (default: the head of the default branch), syncs to it, then pins the commit in `rev`. It refuses a revision whose hooks run a `harness` subcommand the installed one does not know, and names the `cargo install` to run first (see below). |
| `check` | fails when a managed file was modified or deleted, the git review gate is missing, or a hook runs a `harness` subcommand the installed one refuses. It warns when `.claude/settings.json` lacks a deny rule of the review waiver, runs a missing hook or leaves the attribution on, and when Claude Code keeps the transcripts less than a year (`sync` and `update` warn too). |
| `run` | runs a tool of the harness with the binary of the pinned revision, fetched (release asset) or built once into the cache; `harness run` alone lists the tools. |

With a local directory source (this repository), `harness run` builds the source's crate with `cargo`, so a hook runs
the tools of the working tree, or of the last build that compiled.

### The installed launcher

The installed `harness` is the launcher of every project of the machine, not a binary of the pinned revision. A
revision whose hooks call a subcommand it does not know needs its `harness` installed first, or every hook refuses and
the agent can run nothing, the install included. `harness update` refuses such a revision and prints the install to
run; in this repository, `just check-harness` says so before an issue branch is merged (`issue-workflow`, "Merge back").

## Evaluating the harness

The harness is evaluated from the Claude Code session transcripts, over several of its revisions. Claude Code deletes
them after 30 days by default, for every project at once: keep them a year in your user settings
(`~/.claude/settings.json`, or `settings.json` in `CLAUDE_CONFIG_DIR`), which `harness` never writes:

```json
{ "cleanupPeriodDays": 365 }
```

The skill `evaluate-harness` runs the readers below and reports the trend per axis; each can also run alone.

| Tool | Prints |
|---|---|
| `session-metrics [--archive <file>]` | one row of numbers per session (cost, user interventions, project, issue, harness revision, model, version), no message text; `--archive` also merges them into a file kept outside the repositories, so the history outlives the transcripts |
| `rule-breaches [--summary]` | the breaches of the rules a program can see (no `--no-verify`, a review before each commit, no edit of a managed copy, the validation stop of an issue, the commit message format and no AI attribution, the conversation language), apart from what a gate blocked; `--summary` counts them per rule and harness revision |
| `delivered-quality [repo] [--summary]` | one row per done issue: its commits and deliverables, the fixes inside it, the fixes and reverts of its files in the 30 days after its close, its reviews and their findings, the failed checks and typed messages of the sessions that made it; `--summary` sums them per harness revision |
| `fixed-context` | per project and harness revision, the bytes of `CLAUDE.md`, `AGENTS.md` and the skill descriptions that every session pays, and the skills the revision ships |
| `harness-report --sessions <file> [--breaches <file>] [--quality <file>]... [--context <file>]` | one markdown table per axis (efficiency, quality, rules, skills, fixed context), a row per group of consecutive revisions with enough sessions (`--min-sessions`, 10), the model, version and project mix next to the numbers (`--anonymous` keeps only the count of projects) |

## Parallel issues in worktrees

For an issue run in parallel, `just start-issue issues/<name>.md` creates the worktree `.claude/worktrees/<stem>` on
a branch `<stem>`, inside the main one, and makes git ignore it; launch a plain `claude` there. The steps are in
[workflow.md](workflow.md#parallel-issues).

The tools that run on `.` without git's `info/exclude` then read every issue in progress. Keep the worktrees out of
them:

| Tool | Setting |
|---|---|
| measured, prettier, black | `/.claude/worktrees/` in `.gitignore` |
| eslint 9 | `{ ignores: [".claude/worktrees/"] }` in `eslint.config.js` |
| vitest | `exclude: [...configDefaults.exclude, ".claude/worktrees/**"]` |
| jest | `"<rootDir>/.claude/worktrees/"` in `testPathIgnorePatterns` and `modulePathIgnorePatterns` |
| rg, tsc, pytest, ruff, mypy | nothing, they already skip it |
