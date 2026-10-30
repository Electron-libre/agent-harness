---
rank: 12
why: 'before count_the_advisor_calls_of_each_session: a small change the user asked for, that every close benefits from; nothing depends on it'
---
# Link the next issue when closing one

# Goal
When the agent closes an issue with `close-issue`, its report must give a link to the next issue to work on, so the
user can start it without running `just next-issue` by hand. Today the close ends on the move and the commit, and
the next issue is left for the user to look up.

# Findings
* `crates/cardinal/src/tools/close_issue.rs` prints `moved ... -> ...`, `removed ... from queue.md` and the
  rewritten references; nothing about what comes next.
* `crates/cardinal/src/tools/next_issue.rs` computes the next free issue (lowest `rank`, not taken by a branch or a
  worktree, PAR-4) and prints `Next: issues/<stem>.md` with its title and `why`; its ordering is private to the
  module.
* `files/.claude/skills/issue-workflow/SKILL.md`, Rules: the close (`just close-issue issues/<name>.md`, final
  commit, milestone set `done`, release proposal) says nothing on the next issue; the final report of the autonomous
  mode is `deliverable | commit`, then `left`.
* In a linked worktree, `close-issue` is run by the user in the main worktree after the merge
  (`details.md`, "Merge back"): the agent's report then can't hold what `close-issue` prints, only the commands
  left to the user.
* `specs/close-issue.md` (CLS-1 to CLS-6) covers the move and the references only.

# Approach
1. **Deterministic**: after a successful move, `close-issue` prints the next free issue as `next-issue` does
   (`Next: issues/<stem>.md` and its title), `No open issue.` or `Every open issue is taken.`; the selection of
   `next_issue.rs` is shared, not copied. The closed issue is already in `issues/done/`, so it can't be offered.
   A new requirement CLS-7 in `specs/close-issue.md`, proved by a test of `tests/close_issue.rs`.
2. **Rule**: `skill:issue-workflow`, Rules: after `close-issue`, the final report ends on a Markdown link to that next
   issue (`[<title>](issues/<stem>.md)`), or on the line saying there is none; in "Merge back", the commands left to
   the user are followed by the next issue the main worktree will offer once the closed one is gone.
3. To weigh at step 1: whether the link alone is enough or the `why` is worth a line too (the reports rule asks
   for one fact per line, no recap).

# Specs
* `specs/close-issue.md`: new CLS-7 (the next issue printed after the move).

# Milestone
Longer autonomy

# Resources
* `crates/cardinal/src/tools/close_issue.rs`, `crates/cardinal/src/tools/next_issue.rs`.
* `specs/close-issue.md`, `specs/next-issue.md` (NXT-1), `specs/parallel-issues.md` (PAR-4).
* `files/.claude/skills/issue-workflow/SKILL.md` (Rules, Autonomous mode) and its `details.md` ("Merge back").
* `.claude/rules/reports-to-the-user.md`.

# Hints
* `close-issue` refuses in a linked worktree: the next issue it prints is computed in the main worktree, where
  `next-issue` sees every branch and worktree.
* Edit `files/`, then `just sync`; `just check-cardinal` fails on a drifted copy.
