# List the repository files through git

# Goal
The issue worktrees now nest in the main one, under `.claude/worktrees/` (ADR 0008). Every script that walks the
repository tree has to skip that directory by hand, and the next one that forgets reads, or rewrites, the files of
every issue in progress. List the files through git instead, which already honours the `info/exclude` line that
`start-issue` writes, and say how a consumer keeps its own tools out of the nested worktrees.

# Findings
* `/code-review high` of the issue `create_issue_worktrees_under_claude_worktrees` (2026-10-27): `close_issue.rs`
  and `check_specs.rs` each hard-code a skip of `.claude/worktrees` of the root they walk, and ADR 0008 records
  that a new walker must do the same.
* `close_issue.rs` also skips `.git`, `target`, `node_modules` and the `skipped_dirs` of `.claude/harness.toml` by
  directory name; `check_specs.rs` skips `target` and `.claude/worktrees` by full path.
* The same review: the tools of a consumer that walk the filesystem without git's ignore rules (test runners,
  linters, formatters, type checkers run on `.`) now see the nested worktrees in the main one. Not measured on a
  consumer yet.

# Approach
1. List every walker of the repository tree in `files/` and `crates/` (the two above, and any other found).
2. Compare a shared helper over `git ls-files -co --exclude-standard` (tracked and untracked, without the ignored
   files, nested worktrees included) with the current walks: untracked documents still found, behaviour outside a
   git repository, the `skipped_dirs` setting kept or dropped.
3. For the consumer tools: measure in a test consumer which tools read `.claude/worktrees/`, then choose between a
   note in the docs for the project owner and a seed default; the shared files stay generic.

# Specs
* `specs/parallel-issues.md`: PAR-14 (the scanners skip `.claude/worktrees/`) may be restated as "the scanners list
  the files through git".

# Resources
* `docs/adr/0008-issue-worktrees-nest-under-claude-worktrees.md`, `specs/parallel-issues.md` (PAR-13, PAR-14).
* `files/.claude/skills/issue-workflow/close_issue.rs` (`text_files`), `files/.claude/specs/check_specs.rs` (test
  file collection) and their tests.

# Hints
* `git ls-files` lists a nested worktree as nothing when it is ignored, but as an embedded repository when the
  `info/exclude` line is missing (a clone that never ran `start-issue`): check both cases.
* `close_issue.rs` rewrites untracked documents too (`-o` is needed, not only the tracked files).
