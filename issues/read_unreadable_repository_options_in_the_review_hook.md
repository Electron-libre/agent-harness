# Refuse unreadable repository values in the review hook

# Goal
`require_code_review.rs` must not let a commit pass on the review of the shell directory when the repository it lands in
is named by a value the text of the command does not show: `--git-dir=$X`, `--work-tree=$X`, `-C "$(mktemp -d)"`, or a
fish `set -x GIT_DIR $X`. The hook must refuse such a line (fail closed), as `REV-7` does for `GIT_DIR=$X`.

# Findings
* `REV-7` refuses an unreadable value of `GIT_DIR` and `GIT_WORK_TREE` assigned in the line
  (`undeterminable_variable`). The options `--git-dir`, `--work-tree` and `-C` are read by `commit_dirs`, which takes
  a literal `$X` as a relative path: the target is then a missing directory and the commit passes, as a target that
  is not a repository does.
* `set -x GIT_DIR $X` (fish, the shell of the user) sets the environment without a `NAME=value` word, so neither the
  environment reader nor the unreadable-value check sees it.
* A variable exported by an earlier command of the session stays out of reach (non-goal of the spec).
* Found by the code review of the last deliverable of `read_git_dir_and_git_work_tree_in_the_review_hook`.

# Approach
1. Add failing cases to `test_require_code_review.rs`: each form above with every repository reviewed, expecting exit 2;
   and literal values (`--git-dir='$X'`, a path without `$`) still passing.
2. Candidate solutions, can be combined:
   * apply the check of `undeterminable_variable` (quotes, escapes, `$`, backticks) to the value of the three options;
   * read the fish `set -x` / `set --export` forms into `environment_repository`, with `set -e` as the `unset`;
   * keep one tokenizer for the assignments and the options, since two parsers can disagree.
3. Decide in `issue-workflow` steps 1 to 3, then amend the spec (a requirement `REV-8`, or `REV-7` widened).

# Specs
* `specs/review-gate.md`: a new requirement `REV-8`, or `REV-7` widened; the non-goal stays for earlier commands.

# Resources
* `files/.claude/hooks/require_code_review.rs` (`commit_dirs`, `environment_repository`, `undeterminable_variable`),
  `files/.claude/hooks/test_require_code_review.rs`
* `specs/review-gate.md`, `docs/architecture.md` (the review gate)

# Hints
* The hook must not refuse `git commit -m "$msg"`: only values that name the repository count.
* Editing a hook test with `sed` or a heredoc that holds both `git commit` and `GIT_DIR=$X` is itself refused by the
  installed hook: write the test with the editing tools.
