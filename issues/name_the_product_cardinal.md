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
