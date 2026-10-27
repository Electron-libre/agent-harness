# Which documents to update after a change

Read by `skill:update-docs`.

| Change | Update |
|---|---|
| `harness` command or option | `README.md`, `docs/architecture.md`, `specs/sync.md` |
| Tool added, ported or dropped under `crates/harness/src/tools/`, or a change of `harness run` | `README.md`, `docs/architecture.md` (Tools and their binary), `specs/tool-binaries.md`, the recipes and skills that call it |
| File added, moved or removed under `files/` | `docs/architecture.md` (what is shared), then run `just sync` here and in the consumers |
| New key read from `.claude/harness.toml` | `seed/.claude/harness.toml` (with its comment), `docs/architecture.md` |
| Project-owned file added under `seed/` | `docs/architecture.md` |
| Hook added, renamed or dropped under `files/.claude/hooks/`, or a deny rule of the harness | `entries/.claude/settings.json`, `docs/architecture.md` (Hooks and their declaration), then `just sync` |
