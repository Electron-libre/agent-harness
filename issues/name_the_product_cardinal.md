# Name the product Cardinal

# Goal
Use the name "Cardinal" for the product in the documentation and the descriptions (README title, CLAUDE.md,
`docs/`, crate description, skill texts), while the generic noun "harness" (an agent harness) keeps its meaning.
No file, command or path changes here.

# Findings
* `harness` appears as a whole word in about 940 lines outside `target/` and `issues/done/`, both for the product
  ("the shared harness", "the harness repository") and for the concept; `files/` and its copies double many of them.
* The crate description: "Synchronise the shared agent harness (skills, hooks, scripts) into a project".

# Approach
1. Decide the wording: "Cardinal" for the product, "agent harness" for the concept (e.g. "Cardinal, the shared
   agent harness of the user's projects").
2. Replace the product mentions in `README.md`, `CLAUDE.md`, `docs/`, `crates/harness/Cargo.toml`, then in `files/`
   (then `just sync`).
3. An ADR recording the name and the series of renames that follow.

# Resources
* `README.md`, `CLAUDE.md`, `docs/architecture.md`, `docs/usage.md`, `docs/adr/`
* `skill:update-docs`

# Hints
* After `rename_the_repository_to_cardinal`.
* Text that names a command, file or path (`harness sync`, `harness.toml`) stays as it is: the later issues of the
  series rename them.

# Tracking

Plan: validated

## Understanding
The prose names the product "Cardinal" ("Cardinal, the shared agent harness of the user's projects"); "harness"
stays for the concept and in every command, file, path, tool, skill or metric name (`harness sync`, `harness.toml`,
`evaluate-harness`, `harness_rev`). Out of scope: code, tests, specs, issues.

## Plan
1. [x] Repository docs: ADR 0012 (the name and the series of renames), `README.md`, `CLAUDE.md`, `docs/`, the crate
   description; verified by `just check` and a reread of the remaining `harness` words; ~120 lines.
2. [ ] Shared texts: the product mentions of `files/` and `seed/` (skills, `harness.just` comments, seeded
   `harness.toml` comment), then `just sync`; verified by `just check` and `just check-harness`; ~60 lines.

## Decisions
- Mode: autonomous
- Base commit: d314130970037959ddf460a24fc0f0d0b026c2e5
- "harness revision" in prose becomes "Cardinal revision"; column and metric names stay.
- `specs/` are left to the later issues of the series, which amend them.

## Deliverable 1
- done: ADR 0012 drafted (Accepted); README, CLAUDE.md, `docs/` and the crate description name the product Cardinal.

| check | result |
|---|---|
| tests | pass (`just check`) |
| lint | pass |
| specs | pass |
| docs | `update-docs`: the remaining `harness` words are commands, paths, tool names or the concept |
| code review | low, see below |

- red: none (docs only)
- Code review: no findings
- not verified: -
- decisions needed: -
- Note: `just check` first failed on `audit_sources`: a cached test binary still pointed at the old checkout path;
  touching the sources rebuilt it.
