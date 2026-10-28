# Usage

The reference the [README](../README.md) leaves out. `cardinal help <command>` and `cardinal run` (the list of the
tools) say the same from the binary.

## Commands

```sh
cardinal init <source> [--rev <rev>]  # writes .claude/harness.toml; <source>: a directory (relative to the project) or a git URL
cardinal sync [--dry-run] [--force]
cardinal update [--rev <tag|branch|commit>] [--dry-run] [--force]
cardinal check
cardinal run <tool> [args]
```

| Command | What it does |
|---|---|
| `sync` | copies the managed files (`files/`), seeds the missing project-owned ones (`seed/`), adds the hooks and the allow and deny rules of Cardinal to `.claude/settings.json` (or removes those the source dropped), installs the git `pre-commit` review gate. A managed file edited locally is a conflict: fix it in the source, or `--force` to overwrite it. |
| `update` | git source only: resolves the revision (default: the head of the default branch), syncs to it, then pins the commit in `rev`. It refuses a revision whose hooks run a `cardinal` subcommand the installed one does not know, and names the `cargo install` to run first (see below). |
| `check` | fails when a managed file was modified or deleted, the git review gate is missing, or a hook runs a `cardinal` subcommand the installed one refuses. It warns when `.claude/settings.json` lacks a deny rule of the review waiver, runs a missing hook or leaves the attribution on, and when Claude Code keeps the transcripts less than a year (`sync` and `update` warn too). |
| `run` | runs a tool of Cardinal with the binary of the pinned revision, fetched (release asset) or built once into the cache; `cardinal run` alone lists the tools. |

With a local directory source (this repository), `cardinal run` builds the source's crate with `cargo`, so a hook runs
the tools of the working tree, or of the last build that compiled.

### The installed launcher

The installed `cardinal` is the launcher of every project of the machine, not a binary of the pinned revision. A
revision whose hooks call a subcommand it does not know needs its `cardinal` installed first, or every hook refuses and
the agent can run nothing, the install included. `cardinal update` refuses such a revision and prints the install to
run; in this repository, `just check-harness` says so before an issue branch is merged (`issue-workflow`, "Merge back").
A machine that has only the `harness` of before the rename refuses every `cardinal` hook the same way: the install
(`cargo install --force ...`) puts `cardinal` and its `harness` alias in place of the old binary. The git review gate
runs `cardinal`, else `harness`.

## Evaluating Cardinal

Cardinal is evaluated from the Claude Code session transcripts, over several of its revisions. Claude Code deletes
them after 30 days by default, for every project at once: keep them a year in your user settings
(`~/.claude/settings.json`, or `settings.json` in `CLAUDE_CONFIG_DIR`), which `cardinal` never writes:

```json
{ "cleanupPeriodDays": 365 }
```

`fixed_context_budget = <bytes>` in the `[project]` table of `.claude/harness.toml` bounds the fixed context every
session pays (`CLAUDE.md`, `.claude/CLAUDE.md`, the rules without `paths`, the files they import, `AGENTS.md` and
`.claude/AGENTS.md` only when imported or when neither `CLAUDE.md` file exists, and the skill descriptions): `check`, `sync` and `update` warn when the work tree
exceeds it, naming the largest part, and never fail because of it.

The skill `evaluate-harness` runs the readers below and reports the trend per axis; each can also run alone. With no
projects directory given, they read the `projects/` of the configuration directory and warn when it cannot be read.

| Tool | Prints |
|---|---|
| `session-metrics [--archive <file>]` | one row of numbers per session (cost, user interventions, project, issue, Cardinal revision, model, version), no message text; `--archive` also merges them into a file kept outside the repositories, so the history outlives the transcripts |
| `rule-breaches [--summary]` | the breaches of the rules a program can see (no `--no-verify`, a review before each commit, no edit of a managed copy, the validation stop of an issue, the commit message format and no AI attribution, the conversation language), apart from what a gate blocked; `--summary` counts them per rule and Cardinal revision |
| `delivered-quality [repo] [--summary]` | one row per done issue: its commits and deliverables, the fixes inside it, the fixes and reverts of its files in the 30 days after its close, its reviews and their findings, the failed checks and typed messages of the sessions that made it; `--summary` sums them per Cardinal revision |
| `fixed-context` | per project and Cardinal revision, the bytes of the memory files (`AGENTS.md` apart, counted only when loaded) and the skill descriptions that every session pays, and the skills the revision ships |
| `harness-changes [repo] --sessions <file>` | one row per `Expected-effect: <metric> <up\|down>` line of a commit of the Cardinal source that changes `files/`, with the revisions of the sessions that hold the change |
| `harness-report --sessions <file> [--breaches <file>] [--quality <file>]... [--context <file>] [--changes <file>]` | one markdown table per axis (efficiency, quality, rules, skills, fixed context), a row per group of consecutive revisions with enough sessions (`--min-sessions`, 10), the model, version and project mix next to the numbers (`--anonymous` keeps only the count of projects); with `--changes`, the verdict of each expected effect (`helped`, `hurt`, `no observed effect`, `confounded` when a group is too small or something else changed too, `no data`) |

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
