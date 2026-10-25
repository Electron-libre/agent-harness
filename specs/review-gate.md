---
id: REV
status: implemented
adrs: []
---

# Code review gate

## Intent

The hook `require_code_review.rs` refuses a `git commit` until the diff has been through a code review. The agent
relies on it to not commit unreviewed work, so the gate must judge the repository the commit really lands in, whatever
options `git` is given to name it.

## Requirements

- **REV-1** (deterministic) The hook must decide on the review of the repository a `git commit` targets, not on that of the shell directory: a commit aimed at an unreviewed repository must be refused while the shell directory's repository is reviewed and unchanged.
- **REV-2** (deterministic) The hook must take the target from the directory given by `-C`; several `-C` accumulate as `git` does, each relative one being taken from the one before.
- **REV-3** (deterministic) The hook must take the target from `--work-tree` when given, else from `--git-dir`: the parent of the innermost `.git` component of its path (so `.git/worktrees/x` too), the directory itself when there is none. `--work-tree` wins over `--git-dir`.
- **REV-4** (deterministic) The hook must read `--work-tree` and `--git-dir` as two words or as `--option=value`, with a quoted value or not.
- **REV-5** (deterministic) A relative `--work-tree` or `--git-dir` must be taken from the directory the `-C` options lead to, wherever they stand on the command line (as `git` does), else from the shell directory.
- **REV-6** (deterministic) With `--git-dir` and no `--work-tree`, the hook must also check the repository of the directory the command runs in, since `git` takes it as the work tree and commits its files into the history of the git directory: the commit must pass only when every repository involved is reviewed.

## Scenarios

- (REV-1) Given the shell directory's repository reviewed and unchanged, when the command is `git --git-dir other/.git --work-tree other commit -am x` and `other` holds an unreviewed change, then the commit is refused.
- (REV-3) Given `git --git-dir other/.git commit`, when `other` is unreviewed, then the commit is refused as if the command had run in `other`.
- (REV-5) Given `git --git-dir .git -C other commit`, then `.git` is `other/.git` although the `-C` comes after it, and the target is `other`.
- (REV-6) Given `git --git-dir ../other/.git commit -am x` with `other` reviewed and the shell directory holding an unreviewed change, then the commit is refused; with `--work-tree ../other` added, only `other` counts.

## Non-goals

- The environment variables `GIT_DIR` and `GIT_WORK_TREE`: the hook only sees the text of the command, not the environment of the shell.
- Following `cd dir && git commit`: the shell directory is the one the tool reports.
- A target that is not a repository: the commit is let through, as with `-C`; `git` itself fails on it.
