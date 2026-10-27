# agent-harness

The shared agent harness of my projects (Claude Code skills and hooks, checks and transcript readers run by `harness run`, `just` recipes, CI),
and `harness`, the tool that copies it into each project and tells when a copy drifted.

```sh
cargo install --path crates/harness --locked
cd my-project
harness init ../agent-harness      # writes .claude/harness.toml (or a git URL with --rev)
harness sync                       # managed files copied, project-owned ones seeded, git pre-commit review gate installed,
                                   # the harness's hooks and deny rules added to (or removed from) .claude/settings.json
harness update [--rev <tag|branch|commit>] [--dry-run] [--force]  # git source: sync to it, then pin the commit in `rev`;
                                   # refuses a revision whose hooks run a `harness` subcommand the installed harness
                                   # does not know, naming the `cargo install` of that revision to run first
harness check                      # fails if a managed file was modified or deleted, the git review gate is missing,
                                   # or a hook runs a `harness` subcommand the installed harness does not know;
                                   # warns when .claude/settings.json lacks a deny rule of the review waiver,
                                   # runs a missing hook or leaves the attribution on (sync and update warn too),
                                   # and when Claude Code keeps the transcripts less than a year
harness run <tool> [args]          # run a tool of the harness (`commit-check`, `check-commits`) with the harness
                                   # binary of the pinned revision, fetched (release) or built once into the cache;
                                   # `harness run` alone lists the tools
```

With a local directory source (this repository), `harness run` builds the source's crate with `cargo`, so a hook runs
the tools of the working tree, or of the last build that compiled.

The harness is evaluated from the Claude Code session transcripts, over several of its revisions. Claude Code deletes
them after 30 days by default, for every project at once: keep them a year in your user settings
(`~/.claude/settings.json`, or `settings.json` in `CLAUDE_CONFIG_DIR`), which `harness` never writes:

```json
{ "cleanupPeriodDays": 365 }
```

`harness run session-metrics` prints one row of numbers per session (cost, user
interventions, project, issue, harness revision, model, version), with no message text; `--archive <file>` also merges
them into a file kept outside the repositories, so the history outlives the transcripts.
`harness run rule-breaches [--summary]` lists the breaches of the harness rules a program can
see (no `--no-verify`, a review before each commit, no edit of a managed copy, the validation stop of an issue,
the commit message format and no AI attribution, the conversation language),
apart from what a gate blocked, and counts them per rule and harness revision.
`harness run delivered-quality [repo] [--summary]` prints one row per done issue: its commits and
deliverables, the fixes inside it, the fixes and reverts of its files in the 30 days after its close, its
reviews and their findings, and the failed checks and typed messages of the sessions that made it; `--summary` sums
them per harness revision.
`harness run fixed-context` prints, per project and harness revision, the bytes of `CLAUDE.md`,
`AGENTS.md` and the skill descriptions that every session pays, and the skills the revision ships.
`harness run harness-report --sessions <file> [--breaches <file>] [--quality <file>]...
[--context <file>]` joins the outputs of these scripts into one markdown table per axis (efficiency, quality, rules,
skills, fixed context), a row per group of consecutive revisions with enough sessions (`--min-sessions`, 10), the
model, version and project mix next to the numbers (`--anonymous` keeps only the count of projects).
The skill `evaluate-harness` runs them all and reports the trend per axis.

`just start-issue` puts the worktree of an issue under `.claude/worktrees/`, inside the main one, and makes git ignore
it. The tools that run on `.` without git's `info/exclude` then read every issue in progress: measured, prettier and
black (add `/.claude/worktrees/` to `.gitignore`), eslint 9 (`{ ignores: [".claude/worktrees/"] }` in
`eslint.config.js`), vitest (`exclude: [...configDefaults.exclude, ".claude/worktrees/**"]`) and jest
(`testPathIgnorePatterns` and `modulePathIgnorePatterns` with `"<rootDir>/.claude/worktrees/"`); rg, tsc, pytest,
ruff and mypy already skip it.

Design and decisions: [docs/architecture.md](docs/architecture.md), [docs/adr/](docs/adr/).
