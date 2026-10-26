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
                                   # runs a missing hook or leaves the attribution on
```

Design and decisions: [docs/architecture.md](docs/architecture.md), [docs/adr/](docs/adr/).
