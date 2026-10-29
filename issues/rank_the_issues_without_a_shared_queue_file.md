# Rank the issues without a shared queue file

# Goal
`issues/queue.md` holds the order of every open issue in one file that `create-issue` inserts into,
`close-issue` removes from and `prioritize-issues` rewrites. When several people (or parallel issue worktrees)
create or close issues on different branches, they edit neighbouring lines of the same file and their merges
conflict. Keep one readable order of the issues without that conflict.

# Findings
* Writers of `issues/queue.md`: `skill:create-issue` (step 5, inserts a line at its rank), `close-issue` (removes a
  line), `skill:prioritize-issues` (rewrites the list), `skill:issue-workflow`.
* Reader: `next-issue` (`crates/cardinal/src/tools/next_issue.rs`) prints the first line and reports the drift
  (queue lines without a file, issues missing from the queue).
* No conflict on `issues/queue.md` has been measured yet: the risk comes from the shape of the file, one line per
  issue edited from several branches.
* The user agreed on 2026-10-29 that this is worth doing for a team.

# Approach
1. Measure first: count the merges of the history that touched `issues/queue.md` on both sides, in this repository
   and in a consumer.
2. Candidates:
   1. A rank in the front matter or the first line of each issue file (`priority: <n>` with gaps); `next-issue`
      sorts the files; a branch only touches its own issue.
   2. Keep `queue.md`, but let only the main branch rewrite it (`prioritize-issues`), new issues landing unranked
      until then and `next-issue` listing them.
   3. A git merge driver (`merge=union`) for `issues/queue.md`, keeping the file as it is.
3. Keep the "why here" of each rank, which the queue carries today.

# Specs
* `specs/close-issue.md` if `close-issue` stops editing the queue; a spec of `next-issue` if the order moves into
  the issue files (none exists yet).

# Resources
* `issues/queue.md`, `crates/cardinal/src/tools/next_issue.rs`, `crates/cardinal/src/tools/close_issue.rs`.
* `files/.claude/skills/create-issue/SKILL.md`, `prioritize-issues/SKILL.md`, `issue-workflow/SKILL.md`.
* `docs/adr/0003-one-worktree-per-issue.md`, `specs/parallel-issues.md`.

# Hints
* The queue format is read by consumers' `cardinal`: a change of its meaning raises `cardinal-format` and `FORMAT`
  (ADR 0014) and declares a migration (ADR 0017).
