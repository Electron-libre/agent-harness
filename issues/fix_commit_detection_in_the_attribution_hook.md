# Fix the commit and pull request detection of the attribution hook

# Goal
`block_ai_attribution.rs` must refuse an agent credit in every commit or pull request text it is meant to cover,
and must not scan commands that only mention the words.

# Findings
* Found by the code review of the review-gate hook (two runs): `writes_message` only matches `gh` when `pr` directly
  follows it, so `gh -R owner/repo pr create --body "...<a co-author trailer naming the agent>..."` or `gh --repo x pr edit`
  is not scanned and the credit is not blocked.
* It also matches the word `commit` anywhere in the 12 words after `git`, so `git log --grep commit` is scanned
  as a commit.
* `require_code_review.rs` already parses the subcommand after the global options of `git` (`commit_dir`); the
  same idea applies to `gh` (global flags before `pr`).

# Approach
1. Add failing cases to `test_block_attribution.rs`: `gh` with a flag before `pr`, `git log --grep commit` and
   `git show commit` (must pass through).
2. Find the subcommand after the global options for `git` and `gh`, instead of a fixed window.
3. Consider sharing the parsing with `require_code_review.rs` if the shared files stay single-script.

# Resources
* `files/.claude/hooks/block_ai_attribution.rs`, `files/.claude/hooks/test_block_attribution.rs`
* `files/.claude/hooks/require_code_review.rs` (`commit_dir`)

# Hints
* The shared files hold nothing specific to one project.
* A hook script is a single file run by `rust-script`: a shared module is not free.
