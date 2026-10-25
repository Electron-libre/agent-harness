---
id: REV
status: implemented
adrs: [0004]
---

# Code review gate

## Intent

The hook `require_code_review.rs` refuses a `git commit` until the diff has been through a code review. The agent
relies on it to not commit unreviewed work, so the gate must judge the repository the commit really lands in, whatever
options `git` is given to name it. A native git `pre-commit` hook (installed by `harness`, see `specs/sync.md`) repeats the
judgment from inside the repository, where git has already resolved it.

## Requirements

- **REV-1** (deterministic) The hook must decide on the review of the repository a `git commit` targets, not on that of the shell directory: a commit aimed at an unreviewed repository must be refused while the shell directory's repository is reviewed and unchanged.
- **REV-2** (deterministic) The hook must take the target from the directory given by `-C`; several `-C` accumulate as `git` does, each relative one being taken from the one before.
- **REV-3** (deterministic) The hook must take the target from `--work-tree` when given, else from `--git-dir`: the parent of the innermost `.git` component of its path (so `.git/worktrees/x` too), the directory itself when there is none. `--work-tree` wins over `--git-dir`.
- **REV-4** (deterministic) The hook must read `--work-tree` and `--git-dir` as two words or as `--option=value`, with a quoted value or not.
- **REV-5** (deterministic) A relative `--work-tree` or `--git-dir` must be taken from the directory the `-C` options lead to, wherever they stand on the command line (as `git` does), else from the shell directory.
- **REV-6** (deterministic) With `--git-dir` and no `--work-tree`, the hook must also check the repository of the directory the command runs in, since `git` takes it as the work tree and commits its files into the history of the git directory: the commit must pass only when every repository involved is reviewed.
- **REV-7** (deterministic) The hook must read `GIT_DIR` and `GIT_WORK_TREE` assigned in the command line before `git` (`GIT_DIR=x git commit`, `env GIT_DIR=x git commit`) or by `export` in the same line, as `--git-dir` and `--work-tree` (same rules as REV-3 to REV-6), the options of the command winning over the variables as in `git`; it must refuse the commit when such a value cannot be determined from the text (`$X`, `$(…)`, backticks). An `unset` of these variables, or `env -u`, is not refused.
- **REV-8** (deterministic) The hook must also refuse the commit when the value of `-C`, `--git-dir` or `--work-tree` cannot be determined from the text (`$X`, `${X}`, `$(…)`, backticks outside single quotes), in the two-word and the `--option=value` forms, and when fish sets `GIT_DIR` or `GIT_WORK_TREE` that way (`set -x`, `set --export`, `set -gx`, `set -Ux`). A fish `set` with a readable value is read as `GIT_DIR=value`, and `set -e` as an `unset`. A literal value (single-quoted `'$X'`, a path without `$`) and a `$` outside those values (`git commit -m "$msg"`) are not refused.
- **REV-9** (deterministic) `require_code_review.rs native` (run by the native `pre-commit` hook) must judge the repository git resolved for the commit (the `GIT_DIR` and work tree git passes, or the current directory when it passes none), never the text of a command, and must decide as `check` does for the same repository: same marker, same snapshot, same exemptions (nothing to commit, `issues/` only, no `git`) and same waiver, so a commit one refuses is refused by the other.
- **REV-10** (deterministic) `native` must exit non-zero with a message telling the agent to run the code review when it refuses, and must exit zero otherwise.
- **REV-11** (deterministic) `native` must judge a linked worktree by its own marker and waiver (the git directory of that worktree), and a repository reached through `--git-dir` by that git directory.
- **REV-12** (deterministic) `check` must refuse a `git commit` line carrying `--no-verify` or `-n` (also in a bundle of short options such as `-nm`; `-am` and a `-n` inside a quoted message are not one), and a line that sets `core.hooksPath` (`-c core.hooksPath=…`, `--config-env=core.hooksPath=…`, `git config core.hooksPath …`), since they skip the native hook; the message must say why.
- **REV-13** (deterministic) `check` must refuse a `Bash` command that writes the waiver file (`claude-review-waived`): a redirection outside quotes whose target names it, or a command word that creates or fills files (`touch`, `tee`, `dd`, `truncate`, `cp`, `mv`, `ln`, `install`, `rsync`, `curl`, `sed -i`, also after `sudo` or `env`) among whose words the name stands. A shell, `eval`, `source` or an interpreter (`sh -c`, `python3`, `perl`…) is not refused for naming the file: the hook cannot tell what its text does, so the deterrent covers the writers above only. The refusal must give the exact path of the waiver of the repository (`git rev-parse --git-path`) and say that only the user creates it.
- **REV-14** (deterministic) `check` must not refuse a command that only names the waiver file: a `grep` or `cat`, a quoted commit message (also holding `touch …` or a redirection), a heredoc body.
- **REV-15** (deterministic) The path `git rev-parse --git-path claude-review-waived` gives from a linked worktree must be the waiver `check` and `native` read for that worktree, not the one of the main worktree.
- **REV-16** (deterministic) The hook must take the shell directory of a `git commit` from the `cd` (or `pushd`) commands that precede it on the line, as `-C` is taken: a relative one from the directory before it, an absolute one as it is, a `cd` that runs apart (a subshell `( … )`, a backtick substitution, a background command, a pipeline) not moving the shell after it; `-C`, `--work-tree` and `--git-dir` are then read from that directory. A `cd` whose target cannot be determined from the text (`$X`, `$(…)`, `~`, `cd -`, `cd` alone, `popd`, `pushd +1`) outside such a command must make the commit refused, whatever the reviews.
- **REV-17** (deterministic) The recipe `waive-review` of `harness.just` must create the waiver at the path `git rev-parse --git-path claude-review-waived` gives from the directory it runs in, so that `check` and `native` find it for that worktree (the main one or a linked one, as REV-15), and must print that path. It must change nothing else in the repository.
- **REV-18** (deterministic) `check` must refuse a `Bash` command that runs the recipe: the command word `just` (also after `sudo` or `env`, in braces or a substitution) followed by `waive-review`, whatever the options between them (`just -f x waive-review`, `just --justfile x waive-review`). The refusal must say that only the user waives the review. A command that only names the recipe (`grep`, `echo`, a quoted commit message, a heredoc body) is not refused, as in REV-14, nor is a `just` run that reads without running (`--show`, `--dry-run`, `--summary`, `--list`, `--dump`).

## Scenarios

- (REV-1) Given the shell directory's repository reviewed and unchanged, when the command is `git --git-dir other/.git --work-tree other commit -am x` and `other` holds an unreviewed change, then the commit is refused.
- (REV-3) Given `git --git-dir other/.git commit`, when `other` is unreviewed, then the commit is refused as if the command had run in `other`.
- (REV-5) Given `git --git-dir .git -C other commit`, then `.git` is `other/.git` although the `-C` comes after it, and the target is `other`.
- (REV-6) Given `git --git-dir ../other/.git commit -am x` with `other` reviewed and the shell directory holding an unreviewed change, then the commit is refused; with `--work-tree ../other` added, only `other` counts.
- (REV-7) Given `GIT_DIR=other/.git GIT_WORK_TREE=other git commit -am x` with `other` unreviewed and the shell directory's repository reviewed, then the commit is refused; given `GIT_DIR=$X git commit`, then it is refused whatever the reviews; given `GIT_DIR=a/.git git --git-dir b/.git commit`, then the target is `b`.
- (REV-8) Given `git -C "$(mktemp -d)" commit`, `git --git-dir=$X commit` or `set -x GIT_DIR $X; git commit`, then the commit is refused whatever the reviews; given `git --git-dir='$X' commit` or `git commit -m "$msg"`, then the decision is that of the reviews; given `set -x GIT_DIR other/.git; git commit` with `other` unreviewed, then the commit is refused as for `GIT_DIR=other/.git`.
- (REV-9) Given a commit whose repository the command text does not name readably, when that repository holds an unreviewed change, then the native hook refuses it, since git resolved the repository.
- (REV-11) Given a linked worktree reviewed and the main worktree not, then a commit in the linked worktree passes.
- (REV-12) Given `git commit -m "fix -n"` or `git commit -am x`, then neither counts as `-n`: the decision is that of the reviews.
- (REV-13) Given `echo x | tee .git/claude-review-waived`, then the command is refused; given `python3 - <<'EOF'` whose body names the file, or `sh -c 'touch .git/claude-review-waived'`, then the decision is that of the reviews; given a commit command, a `;`, then `touch .git/claude-review-waived`, the `touch` is refused although the commit message is not.
- (REV-14) Given a commit whose quoted message holds `echo > .git/claude-review-waived`, or `grep -rn claude-review-waived specs`, then the decision is that of the reviews.
- (REV-16) Given the shell directory's repository reviewed and `other` unreviewed, when the command is `cd ../other && git commit -am x`, then the commit is refused; given `(cd ../other && true); git commit -am x`, then the decision is that of the shell directory's repository; given `cd $X && git commit`, then it is refused whatever the reviews.
- (REV-18) Given `just waive-review` or `cd x && just -f y waive-review`, then the command is refused; given `grep -rn waive-review specs` or a commit whose quoted message holds `just waive-review`, then the decision is that of the reviews.

## Non-goals

- The environment variables `GIT_DIR` and `GIT_WORK_TREE` exported by an earlier command of the session: the hook only sees the text of the command, not the environment of the shell.
- A `cd` the line does not run in order (`cd dir || git commit`, a `cd` in a function or a `case` branch) is read as a plain sequence.
- Stopping `--no-verify` or `core.hooksPath` outside the Claude hook, a commit made by another tool, or an edited or deleted `.git/hooks/pre-commit`: the native hook is a last resort, not a barrier.
- A target that is not a repository: the commit is let through, as with `-C`; `git` itself fails on it.
