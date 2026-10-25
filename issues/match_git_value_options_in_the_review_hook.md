# Skip the value of every git global option in the review hook

# Goal
`require_code_review.rs` must see a `git commit` whatever global options precede it, so that the review gate cannot be
bypassed by `git --git-dir X commit`.

# Findings
* `commit_dir` skips `-C <dir>`, `-c <kv>`, `SKIPPED_OPTIONS` and the joined forms `--git-dir=` and `--work-tree=`. In
  `git --git-dir /x/.git commit -m foo` the path word is taken for the subcommand, so no commit is seen and the commit
  is not gated. Same for `--namespace`, `--exec-path`, `--super-prefix` given as two words.
* `block_ai_attribution.rs` already lists these options in `GIT_VALUE_OPTIONS`: the two hooks disagree.

# Approach
1. Add failing cases to `test_require_code_review.rs` for each two-word option.
2. Use the same list as the attribution hook, or share the parsing if single-script files allow it.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`commit_dir`), `files/.claude/hooks/block_ai_attribution.rs` (`GIT_VALUE_OPTIONS`)

# Hints
* A hook script is a single file run by `rust-script`: a shared module is not free.
