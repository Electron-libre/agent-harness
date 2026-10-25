---
id: PAR
status: implemented
adrs: [0003]
---

# Parallel issue isolation

## Intent

Two agents drive two different issues at once, each in its own git worktree and branch, so that neither one's
uncommitted files, index, build artifacts or queue changes leak into the other's work. Merging back stays a
deliberate step run from the main branch.

## Requirements

- **PAR-1** (deterministic) `start-issue` must create a worktree outside the repository, in the sibling directory `<repo>-<stem>`, on a new branch named after the issue stem, and must print its path.
- **PAR-2** (deterministic) `start-issue` must fail, changing nothing, when the issue file is not committed on the main branch (missing, or with uncommitted edits), when the main worktree is detached, when the branch `<stem>` already exists or when the worktree directory already exists.
- **PAR-3** (deterministic) An issue must count as taken when a branch named after its stem exists or a worktree has it checked out; the mark must be derived from git alone, so it is seen from every worktree and needs no file to clean up.
- **PAR-4** (deterministic) `next-issue` must skip the taken issues, print the first free entry of the queue, and list the taken ones with the worktree path when there is one; when every entry is taken it must say so.
- **PAR-5** (deterministic) `next-issue` must keep its current output when no issue is taken, and must work outside a git repository.
- **PAR-6** (deterministic) `finish-issue` must remove the worktree and delete the branch of an issue only when the branch is merged into the main branch, and must otherwise fail, keeping both.
- **PAR-7** (deterministic) `finish-issue` must refuse a worktree with uncommitted changes, and must not remove a worktree other than the one named after the issue stem.
- **PAR-8** (deterministic) `close-issue` must refuse to run in a worktree other than the main one, so that the queue is edited at merge time on the main branch and never on an issue branch.
- **PAR-9** (deterministic) The hooks that use the git directory (the code review gate and its marker and waiver) must resolve it per worktree, so a review recorded in one worktree is not seen by another.

## Scenarios

- (PAR-3) Given a branch `foo` that exists with no worktree, when `next-issue` runs, then `foo` is skipped and reported as taken without a path.
- (PAR-4) Given the queue `a`, `b` with `a` taken, when `next-issue` runs, then it prints `b` as next and reports `a` as taken.
- (PAR-6) Given a branch merged by fast-forward or by rebase, when `finish-issue` runs, then it counts as merged since its tip is an ancestor of the main branch.

## Non-goals

- Pushing, opening pull requests or merging: the merge back is the user's decision.
- More than one agent on the same issue, and sharing uncommitted files between worktrees.
- Sharing `target/` between worktrees: each has its own.
