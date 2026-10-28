# Rename the repository to cardinal

# Goal
The project is renamed "Cardinal". Rename the GitHub repository (`agent-harness` → `cardinal`) and the local
directory, and point every reference to the old URL at the new one, so later renames of the cardinal series start
from the right source.

# Findings
* Origin remote: `git@github.com:Electron-libre/agent-harness.git` (2026-10-28).
* `agent-harness` appears in 2 lines of `README.md` (title and the mermaid `source` subgraph) outside `issues/done/`.
* The consumers' `.claude/harness.toml` name the source by its git URL; GitHub redirects a renamed repository, but
  the redirect breaks as soon as a new repository takes the old name.

# Approach
1. `gh repo rename cardinal`, then `git remote set-url origin` to the new URL.
2. Rename the local directory (`agent-harness` → `cardinal`); the Claude Code project memory is keyed by the path,
   so move it along.
3. Update the source URL in each consumer's `.claude/harness.toml` (or let the redirect work until their next
   `harness update`); decide which.
4. Check the release assets (`release.yml`) and `install_harness.sh` still resolve with the new URL.

# Resources
* `README.md`, `.github/install_harness.sh`, `.github/workflows/release.yml`
* `docs/architecture.md` (source and consumers)

# Hints
* Outward-facing: the GitHub rename and the consumers' changes are confirmed with the user before running.
* Open worktrees of the repository hold the old absolute path: finish them first.
