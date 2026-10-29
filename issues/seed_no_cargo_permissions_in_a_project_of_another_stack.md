# Seed no cargo permissions in a project of another stack

# Goal
The seeded `.claude/settings.json` allows `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt` and `cargo check`
whatever the stack of the project. A Python or TypeScript project gets permissions it does not use and none for its
own tools, so the agent asks before each test run. Seed only stack-neutral permissions, and let the stack ones come
from the project.

# Findings
* `seed/.claude/settings.json` holds the five `cargo` rules next to the `just` and read-only `git` rules.
* The file is seeded (copied once, then owned by the project); the managed entries of
  `entries/.claude/settings.json` are the ones `sync` keeps (ADR 0007, ADR 0013).
* `just` already wraps the project's commands: an agent that runs `just check` or `just test` needs no `cargo`
  rule.

# Approach
1. Drop the `cargo` rules from the seed and rely on the `just` rules, the project adding its own.
2. Or seed by stack: `cardinal init` detects the stack (`Cargo.toml`, `package.json`, `pyproject.toml`) or takes it
   as an option, and writes the matching rules.
3. Either way, existing projects keep the file they own: say in the docs what to remove by hand, if anything.

# Specs
* `specs/sync.md` for the seeding of `init` (SYN-4) if option 2 is taken.

# Resources
* `seed/.claude/settings.json`, `entries/.claude/settings.json`, `crates/cardinal/tests/settings_hygiene.rs`.
* `docs/adr/0007-sync-manages-entries-of-the-seeded-settings.md`, `docs/adr/0013-sync-manages-the-allow-rules-too.md`.
