---
id: SYN
status: accepted
adrs: [0002, 0004, 0005, 0007]
---

# Harness synchronisation

## Intent

A project keeps the shared harness (skills, hooks, scripts) in step with the central repository without merging
by hand, and learns at once when a managed file drifted from it.

## Requirements

- **SYN-1** (deterministic) `sync` must copy every file under the source's `files/` to the same relative path of the project, executable bit included, and `check` must report a changed executable bit as a modification.
- **SYN-2** (deterministic) `sync` must overwrite a managed file that was not modified locally since the last sync when the source changed.
- **SYN-3** (deterministic) `sync` must not overwrite a managed file modified locally, must report it as a conflict and must fail, unless `--force` is given.
- **SYN-4** (deterministic) `sync` must copy a file under the source's `seed/` only when the project does not have it yet, and must never touch it afterwards, except the managed entries of `.claude/settings.json` (SYN-29).
- **SYN-5** (deterministic) `sync` must delete a managed file the source dropped when it was not modified locally, and must keep it, no longer managed, when it was.
- **SYN-6** (deterministic) `sync` must be idempotent: a second run reports nothing and leaves the project byte-identical.
- **SYN-7** (deterministic) `sync --dry-run` must report what it would do and must not write anything, lock included.
- **SYN-8** (deterministic) `check` must list every managed file modified or deleted since the last sync, and nothing else.
- **SYN-9** (deterministic) `init` must create the project's `harness.toml` with the source and the project-owned defaults of the source, and must refuse to overwrite an existing one.
- **SYN-10** (deterministic) `sync` and `check` must fail without deleting anything when the source has no `files/` directory, or when the lock cannot be read or names a path outside the project.

- **SYN-11** (deterministic) `update` must resolve the requested revision (a branch, a tag or a commit; the head of the source's default branch when none is given) to a full commit hash, write that hash to `rev` in the `[harness]` table of `harness.toml` with the requested name in a trailing comment, then run the `sync` against it.
- **SYN-12** (deterministic) `update` must change only the `rev` line of `harness.toml`: every other line and comment stays byte-identical, and a missing `rev` is inserted right after `source`. When the file cannot be edited that way (a quoted or dotted key), `update` must fail changing nothing.
- **SYN-13** (deterministic) `update` must leave the project untouched, `harness.toml` and lock included, when the `sync` would report a conflict (unless `--force` is given), when the revision is unknown or when the source cannot be read.
- **SYN-14** (deterministic) `update` must fail, changing nothing, when the source is a local directory, since it has no revision.
- **SYN-15** (deterministic) `update --dry-run` must report the target revision and what the `sync` would do, and must write nothing.
- **SYN-16** (deterministic) `update` must report the previous revision (or that there was none), the new one and the files changed.
- **SYN-17** (deterministic) When `rev` is a full commit hash that no branch or tag of the source points to any more, `sync` and `update` must try to fetch that commit by its hash, and use it when the source serves it. When the source does not, they must fail changing nothing, with an error that names the pinned commit as gone from the source and says to run `harness update`.
- **SYN-18** (deterministic) `sync` (so `init` followed by `sync`; `init` alone only writes `harness.toml`, the hook is not there yet) must install the native review gate in the hooks directory of the project's git repository: an executable `pre-commit` script that starts with a mark line and runs from the work tree `rust-script .claude/hooks/require_code_review.rs native` when that file and `rust-script` are there (a work tree of an older revision), else `harness run review-gate native`, failing the commit with that exit code; when the launcher cannot run the tool (exit 3, BIN-14), when the work tree has neither the hook script nor `.claude/harness.toml`, or when the program it needs is not on the path (or is a `harness` without `run`), the script must warn on stderr and let the commit through.
- **SYN-19** (deterministic) When a `pre-commit` without the mark already exists, the installation must move it to `pre-commit.user` and the installed hook must run it after the review gate, passing its arguments and failing the commit when it fails; it must never overwrite the user's hook, and must fail changing nothing when a `pre-commit.user` already exists.
- **SYN-20** (deterministic) `sync` must reinstall the native gate when it is missing, no longer executable or different from the managed one, and report nothing when it is up to date (idempotent, as SYN-6); `--dry-run` must report the installation and write nothing.
- **SYN-21** (deterministic) For a linked worktree, or a project whose `.git` is a file, the gate must be installed in the `hooks` directory of the common git directory.
- **SYN-22** (deterministic) When `core.hooksPath` is set for the project, `sync` must install nothing, report that the native gate is not installed and why, and succeed; a project that is not a git repository, or whose source manages no hook running `harness run review-gate` (SYN-29) and has no `.claude/hooks/require_code_review.rs` among its files (an older revision), gets no gate and no error.
- **SYN-23** (deterministic) `check` must report `missing: native review gate` when the project is a git repository whose lock records a managed hook running `harness run review-gate` (even removed from the settings since) or the managed file `.claude/hooks/require_code_review.rs` (an older revision, even deleted since), whose hooks directory lacks the managed gate and where `core.hooksPath` is unset, and `modified: native review gate` when it differs; it must not report it otherwise, nor when the `CI` environment variable is set (a fresh clone of a CI run has no git hooks and nobody commits there).
- **SYN-24** (deterministic) `check` must print `warning: .claude/settings.json lacks the deny rule <rule>` for each rule of REV-19 (`specs/review-gate.md`) that the `permissions.deny` list of the project's `.claude/settings.json` lacks, when the lock records the review gate as SYN-23 says and that settings file exists; the warning goes to stderr and must not make `check` fail. `sync` adds the rules the source manages (SYN-29); this is how a project whose rule was removed by hand, or whose source does not manage it, learns of it.
- **SYN-25** (deterministic) `check` must print `warning: .claude/settings.json runs the missing hook <path>` for each hook command of the `hooks` table of the project's `.claude/settings.json` that names a file under `.claude/hooks/` the project does not have, when that settings file exists; the warning goes to stderr and must not make `check` fail. A hook the source dropped is deleted by `sync` (SYN-5), but the seeded settings keep running it.
- **SYN-26** (deterministic) `check` must print `warning: .claude/settings.json does not turn off the attribution` when the project's `.claude/settings.json` exists and its `attribution` is neither `false` nor an object whose `commit` and `pr` are both empty strings; the warning goes to stderr and must not make `check` fail.
- **SYN-27** (deterministic) `sync` and `update` must print the warnings of SYN-24 to SYN-26 to stderr, once, judged on the project as the sync leaves it, and must not fail because of them: a hook the sync removed is missing, one it added is present. Under `--dry-run` they must judge the project as the sync would leave it, writing nothing. The deleting `sync` is how a project loses a hook its settings still run (SYN-5): it must say so at once, not at the next `check`.
- **SYN-28** (deterministic) `check`, `sync` and `update` must print `warning: the transcripts are deleted after <n> days (cleanupPeriodDays); set it to at least 365 in <user settings>` when the `cleanupPeriodDays` of the user settings (`settings.json` in `CLAUDE_CONFIG_DIR`, or else in `~/.claude`) is missing, not a number or below 365 (`<n>` is 30, the default, when it is missing or not a number), and `warning: <file> sets cleanupPeriodDays to <n> days, below 365` for each of the project's `.claude/settings.json` and `.claude/settings.local.json` that sets a number below 365; the warning goes to stderr, comes out once, must not make them fail, and is not printed when the `CI` environment variable is set. Claude Code deletes the transcripts of every project after this period, with the value of whichever session sweeps: the evaluation of the harness reads them over several revisions.
- **SYN-29** (deterministic) When the source has `entries/.claude/settings.json`, `sync` must add to the project's `.claude/settings.json` each managed entry it lacks: each string of the fragment's `permissions.deny`, appended to the project's list, and each hook object of the fragment's `hooks` table, appended to the `hooks` of the first group of the same event with the same `matcher` (none, `""` and `"*"` counting as one value, since each matches every tool), or else to a new group appended to the event; the keys that are missing are created.
- **SYN-30** (deterministic) The lock must record each managed entry of the source as `sync` leaves it, so that `check` and the next `sync` know which entries of the settings are the harness's.
- **SYN-31** (deterministic) `sync` must remove from the project's settings each entry the lock recorded that the source no longer manages, when the settings still hold it unchanged (JSON equality, key order ignored); a group left with no hook is removed, and so is an event left with no group. A hook whose command or options changed in the source is the removal of the old entry and the addition of the new one, which takes the place of the old one when the settings hold it; a dropped entry the settings no longer hold unchanged is reported as kept, once.
- **SYN-32** (deterministic) `sync` must leave every other key, value and entry of the settings as the project wrote it, key order included, and must not write the file when it adds and removes nothing (SYN-6); when it writes it, the file is JSON indented by two spaces, one key or element per line, ending with a newline.
- **SYN-33** (deterministic) An entry the lock recorded that the settings no longer hold unchanged (removed or edited by the project) and that the source still manages must be reported as a conflict on `.claude/settings.json <entry>` and make `sync` fail like a conflicting file (SYN-3), the entry not added back, unless `--force` is given, which adds it back.
- **SYN-34** (deterministic) `sync` must print each entry it adds or removes as `<outcome>: .claude/settings.json <entry>` (`<entry>` is `deny <rule>` or `hook <event> <matcher> <command>`, the matcher left out when the group has none), and `--dry-run` must print the same lines and write nothing.
- **SYN-35** (deterministic) When the project's `.claude/settings.json` is missing (once the seeds are copied), cannot be read, is not valid JSON or does not have the shape of SYN-29, `sync` must not touch it, must keep the entries the lock recorded and must not fail because of it; a warning on stderr says why when the file exists.
- **SYN-36** (deterministic) `check` must print `missing: .claude/settings.json <entry>` for each entry the lock recorded that the settings no longer hold unchanged, and nothing for the other entries of the settings, nor when the settings are missing, cannot be read, are not valid JSON or are not in the shape of SYN-29.
- **SYN-37** (deterministic) When `init`, `sync`, `update` or `check` fails with an error (not a conflict nor a drift, which exit 1), `harness` must print the error on stderr and exit 3, a code Claude Code does not read as a refusal (it blocks on 2, BIN-14), so that a hook running one of them never blocks the agent.

## Scenarios


- (SYN-3) Given a project whose file was never synced but differs from the source, when `sync` runs, then it is a conflict: the file is not ours yet.
- (SYN-6) Given a project whose file already equals the source but is not in the lock, when `sync` runs, then it is locked and reported as unchanged.
- (SYN-11) Given a project already at the resolved commit, when `update` runs, then `rev` is rewritten unchanged, the `sync` reports nothing and the run succeeds.
- (SYN-12) Given a `rev` line with a trailing comment from an earlier update, when `update` runs, then the comment is replaced by the new requested name.
- (SYN-17) Given an abbreviated hash or a name that does not resolve, the fetch by hash is not tried and the error stays "unknown revision".
- (SYN-19) Given a user's `pre-commit` that runs a linter, when `sync` runs, then the linter runs after the gate, and a refusal of the gate stops the commit before it.
- (SYN-24, SYN-25, SYN-26) Given a `.claude/settings.json` that cannot be read or is not valid JSON, then `check` prints one warning saying so instead of one per rule, hook or attribution, whether or not the review hook is managed.
- (SYN-25) Given a hook command that names no file under `.claude/hooks/` (a `just` recipe, a tool on the `PATH`), then it is not reported.
- (SYN-26) Given an `attribution` whose `commit` is empty but whose `pr` is unset, then the warning is printed: Claude Code adds its default text to the pull requests.
- (SYN-27) Given a source that dropped a hook the settings run, when `sync --dry-run` runs, then it warns that the settings run the missing hook, though the file is still there.
- (SYN-27) Given an `update` that applies, then the warnings of its sync come out once, not once for the dry run that precedes it and once for the sync.
- (SYN-22) Given `core.hooksPath` set to a versioned directory, then `sync` leaves it untouched and says the gate is not installed there.
- (SYN-28) Given user settings that exist but cannot be read or are not valid JSON, then the warning is `warning: <user settings> cannot be read or is not valid JSON; Claude Code pauses the cleanup of the transcripts until it is fixed`, not the retention one: the value in force is unknown.
- (SYN-28) Given neither `CLAUDE_CONFIG_DIR` nor `HOME` set (or both empty), then the warning is `warning: the user settings cannot be located (set HOME or CLAUDE_CONFIG_DIR); the retention of the transcripts is not checked`.
- (SYN-29) Given a project whose settings already hold a managed hook in a group with other hooks of its own, when `sync` runs, then nothing is added and the file is not written.
- (SYN-29) Given a fragment hook whose matcher matches no group of the project, then it is appended in a new group `{"matcher": ..., "hooks": [hook]}` at the end of its event.
- (SYN-31) Given a source that switches a hook from `rust-script` to a prebuilt binary, when `sync` runs on a project that kept the old declaration unchanged, then the new hook object takes the place of the old one in its group, even when it was alone there, one removal and one addition reported.
- (SYN-33) Given a project that deleted its settings and a source that seeds them, when `sync` seeds them again, then the managed entries are added to the new file without conflict: nothing in it is the project's yet.

## Non-goals

- Installing the gate under a `core.hooksPath` the project sets itself.
- Merging a local change with a source change: a drifted file is fixed in the central repository.
- Fetching more than a branch, tag or commit of a git source.
- Making `check` depend on the source: it compares the project with its lock only, so the pin cannot break it.
- Rewriting the user settings: `harness` warns of the retention, the user raises it.
- Reading the managed settings of an organisation, which can override the retention.
- Guaranteeing that a host serves an unreferenced commit: it depends on the host.
- Managing the keys of the settings other than `permissions.deny` and `hooks` (the attribution stays a warning, SYN-26).
- A separate settings file read by Claude Code: it reads one shared file per project (`.claude/settings.json`) besides the personal `settings.local.json`.
