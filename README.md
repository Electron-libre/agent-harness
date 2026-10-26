# agent-harness

The shared agent harness of my projects (Claude Code skills and hooks, `rust-script` checks, `just` recipes, CI),
and `harness`, the tool that copies it into each project and tells when a copy drifted.

```sh
cargo install --path crates/harness --locked
cd my-project
harness init ../agent-harness      # writes .claude/harness.toml (or a git URL with --rev)
harness sync                       # managed files copied, project-owned ones seeded, git pre-commit review gate installed
harness update [--rev <tag|branch|commit>] [--dry-run] [--force]  # git source: sync to it, then pin the commit in `rev`
harness check                      # fails if a managed file was modified or deleted, or the git review gate is missing;
                                   # warns when .claude/settings.json lacks a deny rule of the review waiver,
                                   # runs a missing hook or leaves the attribution on (sync and update warn too),
                                   # and when Claude Code keeps the transcripts less than a year
```

The harness is evaluated from the Claude Code session transcripts, over several of its revisions. Claude Code deletes
them after 30 days by default, for every project at once: keep them a year in your user settings
(`~/.claude/settings.json`, or `settings.json` in `CLAUDE_CONFIG_DIR`), which `harness` never writes:

```json
{ "cleanupPeriodDays": 365 }
```

`rust-script .claude/transcripts/session_metrics.rs` prints one row of numbers per session (cost, user
interventions, project, issue, harness revision, model, version), with no message text; `--archive <file>` also merges
them into a file kept outside the repositories, so the history outlives the transcripts.
`rust-script .claude/transcripts/rule_breaches.rs [--summary]` lists the breaches of the harness rules a program can
see (no `--no-verify`, a review before each commit, no edit of a managed copy, the validation stop of an issue,
the commit message format and no AI attribution, the conversation language),
apart from what a gate blocked, and counts them per rule and harness revision.
`rust-script .claude/transcripts/delivered_quality.rs [repo]` prints one row per done issue: its commits and
deliverables, the fixes inside it, the fixes and reverts of its files in the 30 days after its close, its
reviews and their findings, and the failed checks and typed messages of the sessions that made it.

Design and decisions: [docs/architecture.md](docs/architecture.md), [docs/adr/](docs/adr/).
