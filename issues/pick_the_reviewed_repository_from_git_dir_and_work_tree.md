# Pick the reviewed repository from --git-dir and --work-tree

# Goal
`require_code_review.rs` must check the repository a `git commit` really targets, so that
`git --git-dir /other/.git --work-tree /other commit` cannot pass on the review of the current directory.

# Findings
* `commit_dir` now skips the value of `--git-dir` and `--work-tree` (two words or `--opt=value`) but only `-C` sets
  the directory. The commit is checked against the repository of the shell directory.
* If that repository was reviewed and is unchanged, a commit in another repository passes unreviewed.
* Found by the review of `match_git_value_options_in_the_review_hook`.

# Approach
1. Add failing cases to `test_require_code_review.rs`: a commit given `--git-dir`/`--work-tree` of an unreviewed
   repository while the current one is reviewed.
2. Make `commit_dir` return the work tree (or the parent of the git directory) named by these options, as it does for `-C`.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`commit_dir`, `toplevel`), `files/.claude/hooks/test_require_code_review.rs`

# Hints
* `GIT_DIR` and `GIT_WORK_TREE` set in the environment of the command are not seen either; decide whether they are in scope.
