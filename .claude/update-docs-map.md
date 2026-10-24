# Which documents to update after a change

Read by `skill:update-docs`.

| Change | Update |
|---|---|
| `harness` command or option | `README.md`, `docs/architecture.md`, `specs/sync.md` |
| File added, moved or removed under `files/` | `docs/architecture.md` (what is shared), then run `just sync` here and in the consumers |
| New key read from `.claude/harness.toml` | `seed/.claude/harness.toml` (with its comment), `docs/architecture.md` |
| Project-owned file added under `seed/` | `docs/architecture.md` |
