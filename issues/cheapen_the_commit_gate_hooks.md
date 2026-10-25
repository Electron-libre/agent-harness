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
