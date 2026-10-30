---
rank: 62
why: 'cheap, and each close of an issue named in a roadmap pays a review that cannot find anything; before the chained issues, which close more often'
---
# Let the close of an issue commit without a review

# Goal
The native `pre-commit` hook (`review-gate native`, REV-9) lets a commit through without a review only when its
diff touches `issues/` alone. `close-issue` also rewrites the references to the closed issue in the other documents
(CLS-1, CLS-2), the roadmap first, since every issue of a milestone is linked there. So the commit that closes such an
issue is refused until a `/code-review low` runs on a diff of rewritten links, which it cannot find fault with. The
expected outcome: the commit of a close goes through when its changes outside `issues/` are only the rewrites
`close-issue` made, and is still refused as soon as anything else is in it.

# Findings
* Met on 2026-10-30 at the close of `issues/done/count_the_advisor_calls_of_each_session.md`: the staged diff was the
  `git mv` of the issue, the rewritten references in two other issues and one line of `ROADMAP.md`
  (`issues/<stem>.md` to `issues/done/<stem>.md`); the hook refused it, and the review it required found nothing.
* `records_issues_only` of `crates/cardinal/src/tools/review_gate.rs` checks that every staged path starts with
  `issues/`; nothing else is exempt.
* Setting a milestone `done` in the same commit (the Rules of `issue-workflow`) is an edit of the roadmap by the agent,
  not a rewrite by the tool.

# Approach
1. Exempt a commit whose staged diff, outside `issues/`, holds only changed lines that become equal once each
   `issues/done/<stem>.md` is read back as `issues/<stem>.md`, for a `<stem>` whose move to `issues/done/` is staged
   in the same commit (the relative links of CLS-2 included).
2. Or have `close-issue` record the files and lines it rewrote (e.g. a file under `.git/`), and let the gate accept
   exactly those.
3. Decide whether a milestone set `done` in the same commit stays gated (a review of one word) or goes to a commit of
   its own.

# Specs
* `specs/review-gate.md`: amends REV-9.
* `specs/close-issue.md`: only if option 2 makes `close-issue` record its rewrites.

# Milestone
Longer autonomy

# Resources
* `crates/cardinal/src/tools/review_gate.rs` (`records_issues_only`), `crates/cardinal/src/tools/close_issue.rs`.
* `specs/review-gate.md` (REV-9), `specs/close-issue.md` (CLS-1, CLS-2).
* `files/.claude/skills/issue-workflow/SKILL.md` (Rules: the close), `files/.claude/skills/issue-workflow/details.md`
  ("Review waiver").

# Hints
* The exemption must never let through a change the tool did not make: a link rewrite next to an edited sentence on
  the same line is a review case.
