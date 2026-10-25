# Architecture

## What is shared, and how

A consumer project holds a `.claude/harness.toml` with the `[harness]` table (`source`, and `rev` for a git
source) and its own values (`[project]`, read by the scripts and the skills). `harness sync` reads the source and:

- copies every file under `files/` to the same relative path (managed: scripts, tests, hooks, skills, `harness.just`,
  `AGENTS.md`, `specs/README.md`, the CI workflow);
- copies every file under `seed/` once, when the project lacks it (project-owned: `.claude/harness.toml`
  defaults, `.claude/settings.json`, `.claude/update-docs-map.md`, `justfile`, `issues/queue.md`).

`.claude/harness.lock` holds the hash of each managed file as written. A managed file whose hash differs from the
lock was edited locally: `sync` reports a conflict instead of overwriting it, and `harness check` fails. The fix
belongs in this repository, then `sync` everywhere. There is deliberately no merge.

## Making files identical

Everything that differs between projects is a value, not code. Scripts read lists from `.claude/harness.toml`
(`commit_scopes`, `root_docs`, `skipped_dirs`) with a std-only reader; skills tell the agent which key to read
(`roadmap`, `architecture`, `test_conventions`, `confidential`, `user_language`). The change-to-documents table of
`update-docs` lives in the project's `.claude/update-docs-map.md`. Anything else that is project-specific
(quality benchmark, smoke test, the language-specific recipes of the `justfile`) stays in the project.

## Hooks and their declaration

The hooks are managed (`files/.claude/hooks/`) but their declaration lives in `.claude/settings.json`, which is
seeded once, so a new hook reaches a project that already adopted the harness only when the declaration is added
to its file by hand. `harness check` stays lock-only, so the hygiene test of the settings (`test_settings.rs`, run by
`just test-harness` and by CI) fails for a hook script nobody declared, and its message gives the event, the matcher
(the `Declare:` line of the hook's header) and the command to add. An agent that updates the harness runs
`just test-harness` afterwards and is told what to add. The declaration of the attribution hook, under `hooks`
in `.claude/settings.json`:

```json
"PreToolUse": [
  {
    "matcher": "Bash",
    "hooks": [
      { "type": "command", "command": "rust-script \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/block_ai_attribution.rs" }
    ]
  }
]
```

`block_ai_attribution.rs` refuses a `git commit` or `gh pr create|edit` whose message (inline, heredoc, `-F` or
`--body-file`) credits the agent: a co-author trailer, a "Generated with" footer or a credit line ending with the
vendor's no-reply address. It finds the subcommand after the global options (`git -C dir commit`,
`gh -R owner/repo pr create`), so `git log --grep commit` is not scanned; an option that takes a value and is unknown to
its lists hides the subcommand and the message is not scanned. It reads only the literal command and message
files: a message built by `$(cat file)` or a shell variable is out of reach. Its patterns are narrower than those of `commit_check.rs` on purpose, so a
message that discusses the rule passes.

`require_code_review.rs` makes the code review of `issue-workflow` (step 4.5) a precondition of the commit. It is
declared twice, one mode per event: `record` on `PostToolUse` `Skill`, `check` on `PreToolUse` `Bash`:

```json
"PostToolUse": [
  { "matcher": "Skill", "hooks": [
    { "type": "command", "command": "rust-script \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/require_code_review.rs record" } ] }
]
```

When the `code-review` skill ends (it runs forked, so the hook fires at the end of the review), `record` writes
the marker `claude-code-review.json` in the git directory, never a file of the work tree: the `HEAD`, a hash of
the working tree against `HEAD` (tracked changes and untracked files, what an add of everything followed by a
commit commits) and the files it covers. At a commit (`git -C dir` is followed), `check` recomputes the snapshot
and exits 2, telling the agent to run the review, when the marker is missing, was made before the last commit, or
the diff holds a file the review never saw. A fix to a file the review saw passes. A tree with nothing to commit,
a diff under `issues/` only and a directory outside a repository are let through. Documentation is reviewed like
code. The user waives the review of the next commit with `touch "$(git rev-parse --git-path claude-review-waived)"` (right in a linked worktree too); the hook stamps the file
with `HEAD` when it lets a commit through and removes it at the first check after `HEAD` moved, so a command that does
not commit (`--dry-run`, a refusal by another hook) leaves it usable, and refuses a Bash command that writes the
file (redirection, `touch`, `tee`, `cp`, a shell), not one that only names it (`grep`, a commit message). That deters the agent from creating one but does not stop a write made by another tool. The hook sees the review
text in the `Skill` response but does not parse it.

The commit is judged on the repository it targets (spec [`specs/review-gate.md`](../specs/review-gate.md)): the work
tree of `--work-tree`, else the parent of the innermost `.git` of `--git-dir` (the directory itself without one), else
the directory the `-C` options lead to, which accumulate as in `git`. With `--git-dir` alone the directory the command
runs in is checked too, since `git` commits its files into that git directory. `GIT_DIR` and `GIT_WORK_TREE` count as
the two options (which win over them) when the line sets them before `git`, after `env`, or with `export`, until an
`unset` or an `env -u` (the fish `set -x NAME value` and `set -e NAME` count the same); a line that gives one a value
the text does not show (`$X`, `${X}`, `$(…)`, `(cmd)`, backticks) is refused, and so is a `-C`, `--git-dir` or
`--work-tree` whose value is one of those. The hook reads the text of the command only: a variable exported by an
earlier command of the session (an `export` in one call, a plain `git commit` in the next) is not seen, so the gate
cannot cover that case.

A third mode, `native`, is run by a git `pre-commit` hook that `harness sync` installs
([ADR 0004](adr/0004-review-gate-moves-from-the-command-to-the-repository.md), `crates/harness/src/gate.rs`;
`.git/hooks` is neither versioned nor cloned, so each clone needs its own `sync`). The installed file is a small `sh`
script, marked by its second line, that starts `rust-script .claude/hooks/require_code_review.rs native` from the
work tree and then the `pre-commit` the user already had, which `sync` moved to `pre-commit.user` (it never overwrites a
hook that is not its own, and stops before writing anything when `pre-commit.user` exists too). A missing review hook or
`rust-script` is a warning on stderr, not a refusal. `sync` reinstalls the script when it is missing, edited or no longer
executable, and `--dry-run` only reports. `init` writes `harness.toml` only: the review hook is not there until the first
`sync`, which installs the gate.

The gate goes to the hooks directory of the repository whose top is the project, so a linked worktree (or a `.git` that
is a file) installs it once in the hooks of the common git directory, for all its worktrees; a project nested in another
repository, a directory outside any repository and a source without the review hook get no gate and no error. When
`core.hooksPath` is set, git reads its hooks elsewhere, often a versioned directory: `sync` then installs nothing,
reports `skipped (…)` with the line to add to the user's own `pre-commit`, and still succeeds.
Git runs the hook in the repository that really receives the commit, with `GIT_DIR` and the work tree already
resolved, so `native` reads no event and no command: it takes the same marker, snapshot, exemptions and waiver as
`check` from that repository. The temporary index of `git commit -a` (`GIT_INDEX_FILE`) is ignored so the snapshot is
the same.

A native hook does not run under `--no-verify` or `-n`, nor once `core.hooksPath` points elsewhere, so `check` refuses
a commit that carries `--no-verify` (or an abbreviation) or `-n` (alone or in a bundle such as `-anm`), and any command
that sets `core.hooksPath` (`-c`, `--config-env`, `git config`, `GIT_CONFIG_KEY_n`). The value of `-m` and of the other
options that take one is not read, and `--get` is let through. Only the Claude hook can refuse these; a commit made by
another tool is out of reach.

## The instruction audit

`audit-instructions` is the one skill that reads outside the repository: the user's session transcripts, memory
and instruction files under `~/.claude/projects`, through `audit_sources.rs`. It only proposes skills and hooks;
the report holds private content, so it stays in the conversation (or a file outside the repository) and each
validated proposal becomes an issue in generic wording. It is a skill rather than a `harness` subcommand
because telling a repeated procedure from a rule needs judgment, and a skill keeps the shared files free of
project specifics.

## Parallel issues

Two agents can drive two issues at once: [ADR 0003](adr/0003-one-worktree-per-issue.md), spec
[`specs/parallel-issues.md`](../specs/parallel-issues.md). `issue_worktree.rs` (`just start-issue`,
`just finish-issue`) gives an issue the worktree `../<repo>-<stem>` on a branch `<stem>`: separate working tree and
index, shared object store, so one agent's uncommitted files and commits never reach the other. Each worktree has its
own `target/` (measured once, on this repository and a 24-core machine: 239 MB and a cold build in 4 s; a larger
project may want a shared `CARGO_TARGET_DIR`, which is then the project's choice). The mark "taken" is the worktree itself, or a branch holding commits that are not on the main branch:
`next_issue.rs` reads `git for-each-ref`, `git worktree list` and `git cherry`, so every worktree sees it and no file
needs cleaning. A leftover branch with neither is listed `Stale:` and `just finish-issue` cleans it (merged by
ancestry or patch equivalence, `--force` for a squash of several commits). The queue is edited only at merge time: `close_issue.rs` refuses in a linked worktree. The hooks need
nothing: `require_code_review.rs` keeps its marker and waiver in the git directory of each worktree
(`git rev-parse --git-path`), and `.claude/` is tracked, so each worktree holds its own copy of the harness (a project that does not track `.claude/` gets bare worktrees).

## Sources

A directory (relative to the project) or a git URL; a git source is cloned in `$XDG_CACHE_HOME/harness` and
checked out at `rev` (default: the remote's default branch). `rev` pins a commit, not a moving name:
see [ADR 0002](adr/0002-rev-pins-the-resolved-commit.md). `harness update [--rev <name>]` resolves a name
(default: the default branch head) to a commit, previews the sync against it (`--dry-run` stops there), and only
without conflict (or with `--force`) syncs and writes the commit to `rev`; a conflict, an unknown revision or an unreadable source leave the
project as it was. A local directory has no revision and is refused.
A pinned commit that a force-push or a deleted branch left without any ref is fetched by its hash (`git fetch
origin <hash>`); when the source no longer serves it, `sync` and `update` fail naming the commit and `harness
update`. `check` compares the project with its lock and never reads the source; besides, it reports
`missing: native review gate` (or `modified:`) when a project that manages the review hook lacks the managed git hook,
unless `CI` is set, since a fresh clone has no git hooks. The CI step installs the tool
with `cargo install --git --rev <hash>`, which depends on the same host behaviour: it can fail the same way, and the
fix is the same `harness update`.
