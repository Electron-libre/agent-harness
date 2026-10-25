# Install a native git pre-commit hook as the last-resort review gate

# Goal
`harness init` and `harness sync` must install in each consumer a native git `pre-commit` hook that refuses a commit
whose repository holds a change the last code review never saw. Git runs it with `GIT_DIR` and the work tree already
resolved, so the gate no longer depends on reading the repository out of the text of the command, which
`require_code_review.rs` can only approximate.

# Findings
* `require_code_review.rs` parses the command line to find the repository a `git commit` lands in (`-C`, `--git-dir`,
  `--work-tree`, `GIT_DIR=`, `export`). Every new way to name a repository (`$X`, `$(…)`, fish `set -x`) is a bypass
  until the parser learns it.
* A `pre-commit` hook runs in the repository that really receives the commit, whatever the command looked like.
* It does not replace the Claude hook: `--no-verify` and `core.hooksPath` skip a native hook, and only the Claude hook
  can refuse them.
* Follow-up of `read_unreadable_repository_options_in_the_review_hook`, where validating the commit after its creation
  and rolling back was considered and rejected (the post-commit hook cannot tell which repository to reset).

# Approach
1. Decide how the hook reads the review marker of its repository (the same marker and snapshot as
   `require_code_review.rs`, shared code or a single binary entry such as `harness check-review`).
2. Install it from `harness init` / `harness sync`: a managed file under the git hooks directory, chained with a
   `pre-commit` the user already has (never overwritten silently), and re-installed when it drifts.
3. Cover the repositories reached through `--git-dir` and linked worktrees (hooks directory of the common git dir).
4. Make the Claude hook refuse `--no-verify`, `-n` and a `core.hooksPath` change on a `git commit` line.
5. Write an ADR: the gate moves from parsing the command to the repository, with the bypasses that remain.

# Specs
* `specs/review-gate.md`: new requirements for the native hook; `specs/sync.md` for its installation.

# Resources
* `files/.claude/hooks/require_code_review.rs`, `specs/review-gate.md`, `specs/sync.md`
* `crates/harness/` (`init`, `sync`, `check`), `docs/architecture.md` (the review gate), `docs/adr/README.md`

# Hints
* `.git/hooks` is not versioned and not copied by `git clone`: the consumer needs `harness sync` to install it.
* A consumer may set `core.hooksPath` itself; say what the tool does then instead of ignoring it.
