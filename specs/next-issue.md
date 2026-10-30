---
id: NXT
status: accepted
adrs: [0019]
---

# Order of the open issues

## Intent

`next-issue` tells the agent and the user which open issue comes next. The order is kept in each issue file, so a
branch that creates, ranks or closes an issue edits only its own file and never conflicts with another branch over
a shared list (ADR 0019).

## Requirements

- **NXT-1** (deterministic) `next-issue` must order the open issues (the `*.md` files of `issues/`, `queue.md` excluded) by the non-negative integer `rank` of their front matter (the lines between a first line `---` and the next `---`, a value in quotes read without them), lowest first, issues of the same rank by stem, and print the first free one (PAR-4) with its title and the `why` of its front matter.
- **NXT-2** (deterministic) An open issue whose front matter has no `rank`, or a `rank` that is not a non-negative integer, must come after the ranked ones, reported on stderr as `issue without a rank: <stem>` with exit code 1.
- **NXT-3** (deterministic) When open issues share a rank, `next-issue` must report each such rank on stderr as `same rank <n>: <stem>, <stem>...`, without changing its exit code.
- **NXT-4** (deterministic) When `issues/queue.md` exists, `next-issue` must take the order from its lines and report its drift as the format 4 did (a queue line without a file, an issue missing from the queue), ignoring the ranks of the files, so a consumer whose source is of the format 4 keeps working.
- **NXT-5** (deterministic) `next-issue --convert` must write, in the issue file of each line of `issues/queue.md` in order, a front matter `rank` of 100 times its position in the queue and `why` of the reason of the line, `why` in single quotes and removed when the line gives no reason, keeping the other keys of an existing front matter and the rest of the file byte-identical, report the queue lines without a file, then delete `issues/queue.md`; in a git worktree other than the main one it must refuse, changing nothing.
- **NXT-6** (deterministic) When `issues/queue.md` exists and an open issue file has a `rank`, `next-issue` must print on stderr a hint naming `just next-issue --convert`, since the skills of the format 5 rank the files that the queue then hides.

## Scenarios

- (NXT-1) Given `b.md` of rank 100 and `a.md` of rank 200, when `next-issue` runs, then it prints `b` as next.
- (NXT-1) Given `b.md` and `a.md` both of rank 100, when `next-issue` runs, then it prints `a` as next.
- (NXT-2) Given `a.md` of rank 100 and `c.md` with no front matter, when `next-issue` runs, then it prints `a` as next, lists `c` after it, reports `issue without a rank: c` and exits 1.
- (NXT-3) Given `a.md` and `b.md` both of rank 100, when `next-issue` runs, then stderr holds `same rank 100: a, b` and it exits 0.
- (NXT-4) Given `queue.md` listing `b` then `a`, and `a.md` of rank 100, `b.md` of rank 200, when `next-issue` runs, then it prints `b` as next.
- (NXT-5) Given `queue.md` listing `- [b](b.md) - needed first` then `a`, when `next-issue --convert` runs, then `b.md` starts with the front matter `rank: 100` and `why: 'needed first'`, `a.md` with `rank: 200`, and `queue.md` is gone.
- (NXT-5) Given the shell in a linked worktree, when `next-issue --convert` runs, then it fails and `queue.md` is unchanged.
- (NXT-6) Given `queue.md` listing `a`, and `b.md` of rank 150, when `next-issue` runs, then it reports `b` missing from the queue and hints at `just next-issue --convert`.

## Non-goals

- Renumbering the ranks when no gap is left: `skill:prioritize-issues` rewrites them.
- Validating the rank in `check-issue`: `next-issue` reports it.
- Converting the queue during `sync` or `update`: the conversion is run once, by the agent the migration warns.
