# Keep the review waiver until the commit really happens

# Goal
The user waives the review of the next commit by creating the waiver file in the git directory. The waiver must be
consumed by the commit it was meant for, not by a command that only looks like a commit.

# Findings
* `refusal` removes the waiver as soon as it sees one, in the `PreToolUse` check. If the command then does not commit
  (`git commit --dry-run`, a commit refused by `block_ai_attribution`, by another hook or by git itself), the waiver is
  gone and the next real commit is refused although the user waived it.

# Approach
1. Add a failing test: waiver present, the check lets a command through, a second check still finds the waiver
   when no commit happened.
2. Candidates: remove the waiver in a `PostToolUse` step once `HEAD` moved; or record `HEAD` at waiver use and drop
   the waiver only when it changed; or ignore `--dry-run`.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`refusal`, `main`), `files/.claude/hooks/test_require_code_review.rs`

# Hints
* Hooks of one event run in parallel or in order depending on the client: do not rely on the order of the two
  `PreToolUse` Bash hooks.
* The hook refuses any Bash command that names the waiver file: write the issue and the tests without that literal in a command.
