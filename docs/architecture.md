# Architecture

## What is shared, and how

A consumer project holds a `.claude/harness.toml` with the `[harness]` table (`source`, and `rev` for a git
source) and its own values (`[project]`, read by the scripts and the skills). `harness sync` reads the source and:

- copies every file under `files/` to the same relative path (managed: scripts, tests, hooks, skills, `harness.just`,
  `AGENTS.md`, `specs/README.md`, the CI workflow);
- copies every file under `seed/` once, when the project lacks it (project-owned: `.claude/harness.toml`
  defaults, `.claude/settings.json`, `.claude/update-docs-map.md`, `justfile`, `issues/queue.md`);
- merges the entries of `entries/.claude/settings.json` (the deny rules and hooks of the harness) into the
  project's `.claude/settings.json`, leaving its other keys and entries alone (see "Hooks and their declaration").

`.claude/harness.lock` holds the hash of each managed file as written, then the managed settings entries. A managed file whose hash differs from the
lock was edited locally: `sync` reports a conflict instead of overwriting it, and `harness check` fails. The fix
belongs in this repository, then `sync` everywhere. There is deliberately no merge.

## Making files identical

Everything that differs between projects is a value, not code. Scripts read lists from `.claude/harness.toml`
(`commit_scopes`, `root_docs`, `skipped_dirs`) with a std-only reader; skills tell the agent which key to read
(`roadmap`, `architecture`, `test_conventions`, `confidential`, `user_language`). The change-to-documents table of
`update-docs` lives in the project's `.claude/update-docs-map.md`. Anything else that is project-specific
(quality benchmark, smoke test, the language-specific recipes of the `justfile`) stays in the project.

## Tools and their binary

The tools of the harness move from `rust-script` files to modules of the `harness` crate
([ADR 0010](adr/0010-the-tools-ship-in-the-harness-binary.md), spec [`specs/tool-binaries.md`](../specs/tool-binaries.md));
`commit-check` and `check-commits` are ported, the other scripts still run under `rust-script`. A hook, a recipe or a
skill calls `harness run <tool> [args]`. The installed `harness` is only a launcher (`crates/harness/src/run.rs`): it finds
the nearest `.claude/harness.toml` from the current directory and replaces itself (`exec`) by
`<binary> tool <tool> [args]`, so input, output and exit code are the tool's (`crates/harness/src/tools/`, one module
per tool, listed in `TOOLS`).

- Git source: the binary of the pinned commit and the platform, `<cache>/harness/bin/<commit>/<target>/harness`. When
  missing, `curl` downloads the asset `harness-<target>` of the release `harness-<commit>` of a GitHub source and its
  `.sha256`; a digest mismatch fails and installs nothing. Any other failure falls back to
  `cargo install --git <source> --rev <commit> --locked`. The file is written next to its place and renamed, so a
  concurrent hook never runs half a binary. `sync` and `update` fetch it ahead (a failure is a warning).
- Local directory source (this repository): `cargo build -p harness` of the source on each call (about 40 ms when
  nothing changed), whose binary also goes to `<cache>/harness/bin/local-<hash of the path>/harness`; when the working
  tree does not compile, that last good build runs, with a warning, so the hooks keep working.

## Hooks and their declaration

The hooks are managed (`files/.claude/hooks/`) but Claude Code runs them only once declared in
`.claude/settings.json`, which is seeded and then owned by the project. The source therefore declares what it manages
in `entries/.claude/settings.json` ([ADR 0007](adr/0007-sync-manages-entries-of-the-seeded-settings.md),
`crates/harness/src/entries.rs`, SYN-29 to SYN-36): the `permissions.deny` rules of the review waiver and the hook
objects of the `hooks` table. `harness sync` appends to the project's settings each such entry they lack (a hook in the
first group of its event with the same matcher, else in a new group), removes the entries the source dropped when the
settings still hold them unchanged, and records the entries in the lock (`entry  <json>` lines). A hook whose command
changes takes the place of the old one in its group (one removal and one addition reported). A recorded entry the project removed or edited is a conflict, like a
modified managed file: `sync` fails unless `--force` adds it back, and `harness check` reports it as
`missing: .claude/settings.json <entry>`. Every other key stays as the project wrote it, in its order; the file is
rewritten (JSON indented by two spaces) only when an entry is added or removed. Settings that are not valid JSON or
not in the shape of Claude Code's are left alone, with a warning. There is no separate file to own instead: Claude
Code reads one shared settings file per project, besides the personal `settings.local.json`.

`harness check` also warns of what the settings lack or run in vain: a deny rule of the review waiver, a hook under
`.claude/hooks/` the project no longer has, an attribution left on. `harness sync` and `harness update` print the same
warnings, judged on the project as the sync leaves it (or would, under `--dry-run`). The hygiene test of the settings
(`test_settings.rs`, run by `just test-harness` and by CI) fails for a hook script nobody declared, and its message
gives the event, the matcher (the `Declare:` line of the hook's header) and the command to add to
`entries/.claude/settings.json` in the source.

No hook guards the attribution: the seeded settings turn it off at the source (`"attribution": {"commit": "", "pr": "",
"sessionUrl": false}`), `harness run commit-check` rejects a credit line in a message (run by `skill:commit-message` and by CI
through `just check-commits`), and `harness check` warns of settings that leave it on.

The transcripts of the sessions are what the harness is evaluated from, across several of its revisions, and Claude
Code deletes them after `cleanupPeriodDays` (30 days by default). The sweep deletes those of every project with the
value of the session that runs it, so only the user settings, which the harness never writes, keep them all.
`harness check`, `sync` and `update` warn, outside CI, when the user settings keep them less than 365 days or a
settings file of the project lowers the period (`settings::retention`).

The Stop hook `clippy_gate.rs` blocks the end of a turn while `cargo clippy` reports a problem in a project with a
`Cargo.toml` and a changed `.rs` file outside the `.claude` directories, whose scripts are not crate code (spec
[`specs/clippy-gate.md`](../specs/clippy-gate.md)). What it prints is billed to the agent at each turn, so it sends one
line per diagnostic (`--message-format=short`: file, line, column, message) and per error with no location, at most
40, never the build progress, and runs clippy only when the Rust state changed since the last passing run: that run
writes a hash of `HEAD` and of the path and content of each changed `.rs` file to `claude-clippy-pass` in the git
directory of the worktree.
The hooks that run cargo do not trust `CLAUDE_PROJECT_DIR`: a session opened in the main worktree that entered an
issue worktree (`EnterWorktree`) keeps it on the main one, where another agent may work. `clippy_gate.rs` checks the
work tree of the event's `cwd`, at the place `CLAUDE_PROJECT_DIR` has in its own (a crate in a subdirectory), and
`format_rust.rs` runs `cargo fmt` from the directory of the edited file.
For the same reason `just test-harness` prints nothing when every test script passes, and only the lines of the
failing script that are not `ok` otherwise (`test_recipes.rs` proves it); the seeded `justfile` asks the same of the
project's `check`.

`require_code_review.rs` makes the code review of `issue-workflow` (step 3.5) a precondition of the commit. It is
declared twice in `.claude/settings.json`, one mode per event: `record` on `PostToolUse` `Skill`, `check` on
`PreToolUse` `Bash` (a third mode, `native`, is run by git, see below):

```json
"PostToolUse": [
  { "matcher": "Skill", "hooks": [
    { "type": "command", "command": "rust-script \"$CLAUDE_PROJECT_DIR\"/.claude/hooks/require_code_review.rs record" } ] }
]
```

When the `code-review` skill ends (it runs forked, so the hook fires at the end of the review), `record` writes
the marker `claude-code-review.json` in the git directory, never a file of the work tree: the `HEAD`, a hash of
the working tree against `HEAD` (tracked changes and untracked files, what an add of everything followed by a
commit commits) and the files it covers. The hook sees the review text in the `Skill` response but does not parse it.

The gates catch the mistakes of a cooperative agent, not deliberate bypasses
([ADR 0005](adr/0005-the-gates-catch-mistakes-not-bypasses.md)): the review is judged by the native hook below, on the
repository git resolved, and `check` reads the text of a command only for what would skip that hook. The user waives
the review of the next commit with `just waive-review` (it creates `claude-review-waived` at the path
`git rev-parse --git-path` gives, right in a linked worktree too). The `permissions.deny` rules that `harness sync`
manages in `.claude/settings.json` (`Bash(just waive-review)`, `Bash(just waive-review *)`,
`Edit(//**/claude-review-waived)`) make Claude Code refuse the agent the recipe and an edit of the file; they match the
command the agent usually writes, not a `touch` or a script. `harness check` still prints a `warning:` line for each
rule the settings lack (`DENY_RULES` of `crates/harness/src/settings.rs`), whatever the source manages.

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
resolved, so `native` reads no event and no command (spec [`specs/review-gate.md`](../specs/review-gate.md)). It
recomputes the snapshot and exits 2, telling the agent to run the review, when the marker is missing, was made before
the last commit, or the diff holds a file the review never saw. A fix to a file the review saw passes. A tree with
nothing to commit and a diff under `issues/` only are let through. Documentation is reviewed like code. A waiver is
stamped with `HEAD` when it lets a commit through and removed at the first run after `HEAD` moved, so a command that
does not commit leaves it usable; a waiver that cannot be read or stamped waives nothing, and a refusal tells the agent to ask
the user to fix it. The temporary index of `git commit -a` (`GIT_INDEX_FILE`) is ignored so the snapshot is the one
`record` took.

A native hook does not run under `--no-verify` or `-n`, nor once `core.hooksPath` points elsewhere, so `check` refuses
a commit that carries `--no-verify` (or an abbreviation) or `-n` (alone or in a bundle such as `-anm`), and any command
that sets `core.hooksPath` (`-c`, `--config-env`, `git config`, `GIT_CONFIG_KEY_n`). The value of `-m` and of the other
options that take one is not read, and `--get` is let through. Only the Claude hook can refuse these; a commit made by
another tool is out of reach.

## The instruction audit

`audit-instructions` and `evaluate-harness` are the skills that read outside the repository. The audit reads the
user's session transcripts, memory and instruction files under `~/.claude/projects`, through `audit_sources.rs` (which shares the transcript reading of
the evaluation, below), and the summary of `rule_breaches.rs`, whose rules broken often are hook candidates. It only
proposes skills and hooks;
the report holds private content, so it stays in the conversation (or a file outside the repository) and each
validated proposal becomes an issue in generic wording. It is a skill rather than a `harness` subcommand
because telling a repeated procedure from a rule needs judgment, and a skill keeps the shared files free of
project specifics.

## The evaluation from the transcripts

`.claude/transcripts/session_metrics.rs` prints one tab-separated row of numbers per session of every project
(spec [`specs/evaluation.md`](../specs/evaluation.md)): tokens by kind, turns, tool calls, hook refusals, skill calls,
typed messages split into plain validations and the rest, tagged with the project, the issue branches, the models and
the Claude Code versions, and the harness revision in force: the transcripts do not record it, so it is the `rev` of
the `.claude/harness.toml` committed on the default branch of the project at the start of the session (`self:<commit>`
for this repository, the last commit touching `files/`). An assistant message is repeated on one line per content block with the same usage, so it
counts once per `message.id`; the sub-agent transcripts count with their session. The format moves between versions,
so an unreadable line, an unknown type or a missing field is counted in its own column rather than skewing the others.
No text of a message is printed, so `--archive <file>` can merge the rows into a file outside the repositories
(a session replaced by its new row, a session whose transcript is gone kept): the history then outlives the retention
of the transcripts. The transcript reading (projects, sessions, sub-agents, the typed-message filter, and the
project and harness revision a session resolves to) lives once in `transcripts.rs`, which `audit_sources.rs` and
`rule_breaches.rs` include too ([ADR 0006](adr/0006-transcript-readers-are-managed-scripts.md)); each including
script carries the SHA-256 of the module, since `rust-script` would otherwise keep running the build of the old one.

`.claude/transcripts/rule_breaches.rs` lists where the agent broke a rule a program can check (spec
[`specs/rule-breaches.md`](../specs/rule-breaches.md)): one row per finding, `breach` when the act went through,
`blocked` when a hook or a permission rule refused it, located by transcript line. The rules are read from the tool
calls and their results: `--no-verify`, a commit after a review refusal with no review nor user turn (a typed
message, slash command or `!` command) in between
(the bypass of [ADR 0005](adr/0005-the-gates-catch-mistakes-not-bypasses.md)), the review waiver, an edit of a path of
the project's `harness.lock` (paths made relative to the work tree of the `cwd` of each call, so an issue worktree
entered mid-session counts), and an edit outside `issues/`, `specs/` and `docs/adr/` between a proposed plan and the
user's answer. The shell words of a command exclude its quoted strings and here-documents, so a commit message that
names `--no-verify` is not a use of it. The commit messages (attribution trailer, Conventional Commits with the
`commit_scopes` of the commit itself, as `just check-commit` read them) are read from the git history of the project
instead, since a commit command rarely shows its message: a non-merge commit of a local branch counts with the
latest-starting session of the project whose span holds its author date, and a commit in no session was the user's
(a commit the agent rebased keeps the date the user wrote it; stashes and notes are on no branch). The conversation language is judged per text
block of the agent of 20 words or more once the code is removed, by counting the frequent words of French and English:
a block with more words of the other language than of the `user_language` committed then is a breach. A session whose
project had no harness then is skipped. `--summary` counts the
findings per rule and revision over the sessions where the rule applied, so a rule that rarely applies does not read
as respected.

`.claude/transcripts/delivered_quality.rs [repo]` measures the quality of the delivered work from the git history
instead (spec [`specs/delivered-quality.md`](../specs/delivered-quality.md)): one row per file of `issues/done/`. The
commits of an issue are those of the default branch touching its file, from the first whose version holds
`# Tracking` to the one that moves it to `issues/done/`, except the closes of other issues that only rewrite a
citation. Its files (issue files, `docs/` and `harness.lock` left out) give the `fix` and `revert` commits of the
next `--window-days` (30) after the close; its plan lines give the deliverables, and its `Code review` lines of `# Tracking` (not the rows of the `check | result`
table) the reviews and their findings (the count opens the line, `issue-workflow` step 3.6; a line with no count leaves the sum empty); the revision is the one in force just
before its first commit, so an issue that changes the harness is measured with the harness it was made with. The
sessions of an issue are those of the projects directory that made its commits (the latest-starting session of the
repository whose span holds the commit date, as for the commit rules above, a session on the issue's branch first, so
a parallel session in another worktree does not take it); within each, from the last commit of other work (from its
start on the issue's branch) to the last commit of the issue, the failed `just check…`/`just test…` runs and the typed messages, split
as `session_metrics.rs` splits them (`is_validation`, now in `transcripts.rs`). `--summary` sums the columns per harness revision, in the order the revisions were used (a sum over an empty value
stays empty), the
input of the report by revision. These are signals, not verdicts: a later fix may come from a new need.

`.claude/transcripts/fixed_context.rs` measures what a revision costs in every session before any work (spec
[`specs/harness-report.md`](../specs/harness-report.md)): one row per project and revision used by a session, the bytes
of `CLAUDE.md`, `AGENTS.md` (a symbolic link counted as its target) and the `name` and `description` of each skill
(what Claude Code loads at the start), read
from the tree of the oldest commit of the default branch where the revision is in force (the sync commit of a pin, the
commit itself for `self:`), with the list of the skills it ships. The shared hooks print nothing in a session but their
refusals, already counted.

`.claude/transcripts/harness_report.rs` joins these outputs (it reads their TSV files, never the transcripts) into
one markdown table per axis. The revisions are taken in the order of their first session and cut into groups of
consecutive revisions holding at least `--min-sessions` (10) sessions, since a `self:` revision often covers one issue;
the last group, when short, shows only its size. Next to the numbers (medians per session, so one long session does
not weigh as ten) stand the sample size and the mix of models, Claude Code versions and projects (`--anonymous` keeps
only their count), the confounders a reader must weigh before reading a trend. The other tables come with their input: the quality
signals summed over the revisions of a group and over the repositories given (`--quality` once per repository), the
rules per group with the share of the sessions where they applied that broke them, the skill calls and hook refusals
with the skills shipped and never called (`--context`), and the mean fixed context. The skill `evaluate-harness`
runs the five scripts into a temporary directory and reports the trend per axis next to the confounders; the
numbers are never turned into a pass or a fail, since they would become targets.

## Parallel issues

Two agents can drive two issues at once: [ADR 0003](adr/0003-one-worktree-per-issue.md),
[ADR 0008](adr/0008-issue-worktrees-nest-under-claude-worktrees.md),
[ADR 0009](adr/0009-scanners-list-the-repository-files-through-git.md), spec
[`specs/parallel-issues.md`](../specs/parallel-issues.md). `issue_worktree.rs` (`just start-issue`,
`just finish-issue`) gives an issue the worktree `.claude/worktrees/<stem>` on a branch `<stem>`: separate working tree and
index, shared object store, so one agent's uncommitted files and commits never reach the other. Claude Code enters a
worktree there without asking for an approval; `start-issue` adds `/.claude/worktrees/` to `.git/info/exclude`, and the
scripts that scan the tree, `close_issue.rs` and `check_specs.rs`, take their files from git (`repo_files.rs`,
shared by `#[path]` include as in ADR 0006, `git ls-files -co --exclude-standard`), which never lists a nested
worktree, ignored or not, so the main worktree never sees the nested ones. `close_issue.rs` then drops `.git`,
`target`, `node_modules` and the `skipped_dirs` of `.claude/harness.toml` by directory name; `check_specs.rs`
walks the disk only outside a git repository. A project's own tools run on `.` may still read the nested
worktrees (measured in ADR 0009); the README says how to keep them out. `finish-issue` still removes a
worktree left at the former sibling path `../<repo>-<stem>`. Each worktree has its
own `target/` (measured once, on this repository and a 24-core machine: 239 MB and a cold build in 4 s; a larger
project may want a shared `CARGO_TARGET_DIR`, which is then the project's choice). The mark "taken" is the worktree itself, or a branch holding commits that are not on the main branch:
`next_issue.rs` reads `git for-each-ref`, `git worktree list`, `git cherry` and `git merge-tree --write-tree`, so every worktree sees it and no file
needs cleaning. A leftover branch with neither is listed `Stale:` and `just finish-issue` cleans it (merged by
ancestry, patch equivalence, or a merge that would change no file, which covers a squash of any number of commits; `--force` for work that is not on the main branch). `just finish-issue` also refuses, `--force` or not, when run from inside the worktree it would remove. The queue is edited only at merge time: `close_issue.rs` refuses in a linked worktree and refuses an issue of one, whatever directory it runs from. An agent
handed an issue in the main worktree starts it itself (`issue-workflow`, step 0): `just start-issue`, then Claude
Code's `EnterWorktree` with the path, since a shell `cd` out of the launch directory does not last; the session's
`cwd`, and the `cwd` of the hook events, become the worktree. The hooks act on the work tree of that `cwd` (their
scripts still come from the main worktree, `CLAUDE_PROJECT_DIR`): `require_code_review.rs` keeps its marker and
waiver in the git directory of each worktree (`git rev-parse --git-path`), `format_rust.rs` and `clippy_gate.rs` run
cargo there. `.claude/` is tracked, so each worktree holds its own copy of the harness (a project that does not track `.claude/` gets bare worktrees). At the close, the agent rebases the branch, leaves the worktree (`ExitWorktree`), fast-forwards the main branch, closes the issue and runs `just finish-issue`.

## The cost of a turn

Everything a check prints is read by the agent and billed as tokens, every turn it runs. Measured on this repository
(bytes / 4 ≈ tokens):

| Output | Before | After |
|---|---|---|
| `just check`, success | 86 lines, 5.6 kB | nothing |
| `just test-harness`, success | 267 lines, 12.7 kB | nothing |
| clippy gate, 3 errors | 35 lines, 1.9 kB | 4 lines, 0.4 kB |
| clippy gate, end of turn with no new Rust change | a full clippy run | no run |

The model is the other lever, and the shared settings deliberately do not choose it: it depends on the user's plan
and the project. The setup that costs least for `issue-workflow` is a strong model where judgment matters (steps 1
and 2: preparation in the files, single validation stop; the final `high` review) and a cheaper one for the mechanical part (step 3: test,
code, docs, the `low` review, the commit). Switch with `/model` after the plan is validated; `/model opusplan`
does not fit, since plan mode is read-only and step 1 writes the issue. Forked
skills and sub-agents follow the session model unless the user sets `CLAUDE_CODE_SUBAGENT_MODEL` in their own
settings.

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
