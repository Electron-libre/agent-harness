---
id: QUA
status: accepted
adrs: [0003, 0006]
---

# Quality signals of the delivered issues

## Intent

`.claude/transcripts/delivered_quality.rs` measures what happened to the work of each done issue of a repository
after it was delivered (fixes, reverts) and while it was made (review findings, failed checks, typed messages), one
row of numbers per issue, so the evaluation report compares harness revisions on quality and not only on cost. It
reports signals, not verdicts: a fix after a feature may come from a new need.

## Requirements

- **QUA-1** (deterministic) `delivered_quality.rs [repo] [--projects <dir>] [--window-days <n>] [--summary]` must print a header line then one tab-separated row per file of `issues/done/` in the work tree of `repo` (default the current directory), sorted by `closed`, with the columns `issue`, `revision`, `closed`, `deliverables`, `commits`, `fixes_in_issue`, `fixes_after`, `reverts_after`, `reviews`, `review_findings`, `sessions`, `failed_checks`, `validations`, `other_messages`; `issue` is the file stem. A `--window-days` that is not a whole number from 0 to 3650, or a `--projects` with no value, must be refused with an error and a non-zero exit.
- **QUA-2** (deterministic) The commits of an issue must be the non-merge commits of the default branch (as EVA-8 picks it) that touch `issues/<stem>.md` or `issues/done/<stem>.md`, from the oldest one whose version of that file holds a `# Tracking` heading to the one that adds `issues/done/<stem>.md` (the close commit), both included, except the commits that add another `issues/done/*.md` file and not this one (they close another issue and only rewrite its citations); `commits` is their count and `closed` the author date of the close commit. An issue with no close commit on the default branch has no row.
- **QUA-3** (deterministic) `deliverables` must count the lines of the `## Plan` section of the done issue file (when it has none, of the `# Tracking` section before its first sub-heading, the layout of the older issues) that start with a number, `.`, a space and `[x]` or `[ ]`, a section ending at the next line starting with `#` outside a fenced code block; `fixes_in_issue` the commits of the issue whose subject has the type `fix` or `revert` (`AGENTS.md`).
- **QUA-4** (deterministic) The files of an issue must be the paths changed by its commits, except those under `issues/` or `docs/` and `.claude/harness.lock`; `fixes_after` must count the non-merge commits of the default branch that come after its close commit in the history and are authored at most `--window-days` days (default 30) after it, have the subject type `fix` and change at least one file of the issue; `reverts_after` counts the same way the commits whose subject has the type `revert` or starts with `Revert "`, or whose message cites `This reverts commit <hash>` of a commit of the issue.
- **QUA-5** (deterministic) `revision` must be the harness revision in force one second before the author date of the oldest commit of the issue, as EVA-8 defines it (`transcripts.rs`): the harness the work was done with, not the one the issue may itself change.
- **QUA-6** (deterministic) In the `# Tracking` section of the done issue file, each line holding `Code review` (case-insensitive) and not starting with `|` (a row of the `check | result` table) must count one in `reviews`, the section ending as in QUA-3; `review_findings` sums over them the number `<n>` of the first `<n> finding` or `<n> findings` of the line (0 for `no findings`; what follows may count the fixed ones), and is empty when a line holds neither, since the findings of that review are unknown.
- **QUA-7** (deterministic) The sessions of an issue must be, for each of its commits, the latest-starting session of the projects directory (`--projects`, default `~/.claude/projects`) whose repository (EVA-7) is `repo` and whose span holds the commit's author date (as BRE-9), taken among the sessions whose branches (EVA-6) include the issue's stem when one does (an issue worktree, ADR 0003, while another session works in parallel); `sessions` is their distinct count. The four transcript columns must be empty, not 0, when the projects directory cannot be read.
- **QUA-8** (deterministic) In a session of the issue, an event counts for the issue when its timestamp lies at or before the latest author date of the issue's commits in that session, and after the latest author date, in that session's span, of a commit of the default branch that is not a commit of the issue and comes before the earliest of them (else at or after the session start; always from the session start when the session's branches include the issue's stem); `failed_checks` counts the `Bash` calls whose command holds `just` followed by a word starting with `check` or `test` and whose result is an error, and `validations` and `other_messages` the typed messages split as EVA-4.
- **QUA-9** (deterministic) With `--summary`, the script must print instead a header then one row per `revision`, with the column `issues` (their count) followed by the sums of the numeric columns of QUA-1 over its issues, sorted by the earliest author date of the oldest commits of its issues (the order the revisions were used in); a sum over an empty value is empty.
- **QUA-10** (deterministic) The output must hold no text of a message, of a commit or of an issue: only stems, revisions, dates and counts; `delivered_quality.rs` must carry the `// transcripts.rs sha256: <hex>` line of EVA-13.

## Scenarios

- (QUA-2) Given an issue opened in a `docs` commit, then three commits of its deliverables, then another issue closed by a commit that rewrites a citation in its file, then its own close commit, when the row is printed, then `commits` is 4.
- (QUA-4) Given an issue closed on day 0 that changed `src/a.rs`, when a `fix` commit changes `src/a.rs` on day 10 and another on day 40, then `fixes_after` is 1 with the default window.
- (QUA-4) Given a later `fix` commit that only changes `docs/architecture.md` and `.claude/harness.lock`, when the row is printed, then it is not in `fixes_after`.
- (QUA-5) Given an issue whose first commit changes the pinned revision from `r1` to `r2`, when the row is printed, then its revision is `r1`.
- (QUA-7) Given a session on the branch `alpha` and a later-starting session on another branch both holding a commit of the issue `alpha`, when the row is printed, then the commit counts with the first.
- (QUA-6) Given an issue file with `Code review: 6 findings` and `Code review: no findings`, then `reviews` is 2 and `review_findings` 6; with one more `Code review: fixed the doc`, `review_findings` is empty.

## Non-goals

- Judging an issue: a fix after a feature may come from a new need; the report reads the trend.
- The issues closed before the `issues/done/` convention or in another repository than the one given: run the script once per repository.
- The commits not yet on the default branch (as EVA-8 picks it, `origin/HEAD` first): an issue closed locally and not pushed has no row yet.
- Archiving the rows: the git columns can be recomputed at any time; the transcript columns follow the retention of the transcripts.
