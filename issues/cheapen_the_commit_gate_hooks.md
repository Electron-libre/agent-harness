# Make the commit gate hooks cheap on commands that are not commits

# Goal
The two `PreToolUse Bash` hooks run on every shell command of the agent. They must cost almost nothing when the
command is not a commit, and a commit gate must not read large untracked files.

# Findings (from a code review, to confirm by measure)
* `require_code_review.rs` `main` runs `git rev-parse --show-toplevel` before knowing that the command is a commit;
  the call is only needed after `commit_dir` returns `Some`.
* Each hook is a separate `rust-script` start: two per Bash call.
* `snapshot` reads the whole content of every untracked file and pipes it to `git hash-object`, on each commit
  attempt: slow and memory heavy with large untracked files that are not ignored.

# Approach
1. Measure the added latency of a plain `ls` with and without the hooks, and a commit with a large untracked file.
2. Candidates: test `commit_dir` first; hash untracked files by `git hash-object --stdin-paths`, or by path, size
   and mtime; one script for both hooks if the cost is the start.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`main`, `snapshot`), `files/.claude/hooks/block_ai_attribution.rs`

# Hints
* The marker must still change when an untracked file changes content.

# Tracking
1. [x] Measure the latency of a plain `ls` (no hook, each hook, both) and of a commit with a large untracked file; decide on deliverable 4
2. [ ] Test `commit_dirs` before `toplevel` in `main` (a non-commit command runs no `git`)
3. [ ] Hash untracked files without loading them (`git hash-object --stdin-paths`); the marker still changes with the content
4. [x] (dropped, see Decisions) Merge the two hooks into one script, only if the measure shows the start dominates (else tick as dropped with the figures)

## Decisions
* Mode: autonomous.
* No spec: the issue changes no behaviour of `specs/review-gate.md`.
* The hash of the marker may change value (a review in progress is redone once); its meaning stays.
* Measure (2026-10-25, hooks already compiled in the `rust-script` cache, plain `ls`, mean of 20 runs): `require_code_review.rs check` 4.1 ms,
  `block_ai_attribution.rs` 2.5 ms (a bare fork is 0.07 ms), so 6.7 ms per Bash call; a single `git rev-parse --show-toplevel` is about 1 ms.
  `check` on `git commit` with one 1 GB untracked file: 2.65 s and a 1.9 GB peak memory. Command: pipe a `PreToolUse` event
  (`{"tool_name":"Bash","cwd":…,"tool_input":{"command":…}}`) into `rust-script files/.claude/hooks/<hook>.rs check`.
* Deliverable 4 dropped: merging would save about 2.5 ms per Bash call, which is not worth a coupling of two hooks, a change of the
  declaration and of `harness sync`. The real costs are the early `toplevel` (about 1 ms of the 4.1) and the memory of `snapshot`.
* Merging `block_ai_attribution` into `require_code_review` is accepted if the measure justifies it.
