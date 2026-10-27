---
id: BIN
status: accepted
adrs: [0002, 0010]
---

# Prebuilt tools of the harness

## Intent

The hooks, checks and transcript readers of the harness run in every consumer, Rust or not. They ship as the
`harness` binary of the pinned revision, fetched prebuilt, so a project needs only `harness` and `just`, and runs the
tools of the revision its managed files come from.

## Requirements

- **BIN-1** (deterministic) `harness run <tool> [args]` must run the tool of the revision selected by the `.claude/harness.toml` nearest the working directory (else nearest `CLAUDE_PROJECT_DIR`, when the agent left the project), with the arguments, standard input and working directory it was given, its exit code, standard output and error passed through unchanged.
- **BIN-2** (deterministic) With a git source, `run` must execute the binary cached for the pinned commit and the current platform (`<cache>/harness/bin/<commit>/<target>/harness`, `<cache>` being `XDG_CACHE_HOME`, else `~/.cache`) without starting any download or build when that binary is present.
- **BIN-3** (deterministic) When that binary is missing, `run`, `sync` and `update` must download with `curl` the asset `harness-<target>` of the release `harness-<commit>` of a GitHub source and its digest `harness-<target>.sha256`, and install the binary only when its SHA-256 matches the digest.
- **BIN-4** (deterministic) The binary must appear in the cache by a rename of a complete file, so that a tool run concurrently never executes a partly written binary.
- **BIN-5** (deterministic) When the download fails (no release, no asset for the platform, a source not on GitHub, no network) and `cargo` is on the path, the fetch must build `harness` with `cargo install --git <source> --rev <commit> --locked` (a `git@host:path` source written `ssh://git@host/path`, the form cargo reads) into a temporary root and install it as BIN-4 says.
- **BIN-6** (deterministic) With a local directory source, `run` must build the `harness` crate of that directory with `cargo` and run the result, so the tools of the repository that develops them are those of its working tree.
- **BIN-7** (deterministic) `run` with no tool or an unknown tool must exit 2 and print the names of the tools on stderr.
- **BIN-8** (deterministic) The hook commands of the source's `entries/.claude/settings.json`, the recipes of `files/harness.just`, the commands the skills of `files/` give and the git `pre-commit` gate `sync` installs must call the tools through `harness run`, never through `rust-script`, except the gate in a work tree that still holds the review hook script of an older revision (SYN-18).
- **BIN-9** (deterministic) The `pre-commit` gate must print a warning on stderr and let the commit through when `harness` is not on the path, as it does today when the review hook is missing.
- **BIN-10** (deterministic) A downloaded asset whose SHA-256 differs from its digest must fail the fetch, naming the commit, and leave no binary in the cache.
- **BIN-11** (deterministic) When the download fails and `cargo` is not on the path or its build fails, the fetch must fail with a message naming the commit, the platform and `harness update`, leaving no binary in the cache.
- **BIN-12** (deterministic) With a local directory source whose build fails, `run` must run the last binary built from that source, with a one-line warning on stderr, and exit 3 (BIN-14) with the build's errors only when there is none, so that a working tree that does not compile does not block every hook.
- **BIN-13** (deterministic) `sync` and `update` must print a fetch that fails as a warning on stderr, without failing because of it, since `run` fetches again; under `--dry-run` they fetch nothing.
- **BIN-14** (deterministic) When `run` cannot get the binary of the tool (no `.claude/harness.toml`, a fetch or a build that fails), it must exit 3 with the reason on stderr, a code Claude Code does not read as a refusal (it blocks on 2), so that a hook that cannot run never blocks the agent.
- **BIN-15** (deterministic) `check` must fail with one line per hook command of the project's `.claude/settings.json` whose first word is `harness` and whose second word is a subcommand the `harness` on the path refuses (`harness help <subcommand>` exits non-zero), the line naming the command and saying to install the `harness` of the project's revision, and look at nothing when no `harness` is on the path, as BIN-9.
- **BIN-16** (deterministic) `update` must write nothing and fail when a hook command of the settings it would write names a subcommand the `harness` on the path refuses (as BIN-15 judges it), naming the command and the install of the target, `cargo install --git <source> --rev <commit> --locked`, except under `--dry-run`, which prints the same lines as warnings without failing.

## Scenarios

- (BIN-2) Given a project pinned to commit C whose binary is cached, when a hook runs `harness run review-gate check`
  with no network, then the cached binary runs and its exit code is the hook's.
- (BIN-10) Given a published digest that does not match the downloaded asset, when `sync` fetches it, then `sync` fails
  naming the commit and the cache holds no binary for C.
- (BIN-4) Given two hooks that start at once with no cached binary, when both fetch, then each runs a complete binary.
- (BIN-5) Given a commit pinned before its release was published (`harness update` right after a push), when `cargo`
  is on the path, then the binary is built from the source instead.
- (BIN-14) Given an agent that ran `cd /tmp`, when a hook runs `harness run review-gate check`, then the tool of the
  project of `CLAUDE_PROJECT_DIR` runs; with no project at all, the hook exits 3 and the command goes on.
- (BIN-12) Given this repository with a syntax error in the crate, when a hook runs, then the tools of the last
  successful build run and stderr says the build failed.
- (BIN-6) Given this repository (`source = "."`) with a change to a tool in its working tree, when a hook runs it,
  then the changed tool runs.
- (BIN-15) Given a branch whose settings gained the hook `harness newcmd`, with an installed `harness` that has no
  `newcmd`, when `harness check` of the branch's code runs in its worktree before the merge, then it fails naming the
  hook; once the branch's `harness` is installed, it passes.
- (BIN-16) Given an installed `harness` with no `run` and a target revision whose hooks call `harness run`, when
  `harness update` runs, then nothing is written and the error names `cargo install --git <source> --rev <commit>
  --locked`; after that install, the same `update` succeeds.

## Non-goals

- Windows: the hooks and recipes run under `sh`.
- Running the tools of another revision than the pinned one, or without `harness.toml`.
- Signing the binaries: the digest proves the asset is the one published with the release, not who published it.
- The release workflow itself: it runs on the source's CI and is checked by its own run, not by a test.
- Checking the words after the subcommand (the tool of `harness run`): the pinned binary runs them (BIN-1), not the
  installed one.
- Hook commands that do not start with `harness`, and an installed `harness` older than BIN-15 and BIN-16: it cannot
  refuse what it does not check.
