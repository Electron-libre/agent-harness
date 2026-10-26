# Drop redundant guards and prose tests

# Goal
Some parts of the harness guard what is already guarded, or test wording instead of behaviour, and each costs
maintenance on every change of the skills. Remove them so that what remains is a check that matters.

# Findings
* `files/.claude/hooks/block_ai_attribution.rs` (172 lines) and its test (about 9 KB) refuse an attribution trailer
  in a `git commit` line; `commit_check.rs` already rejects it, run by `skill:commit-message` and by CI through
  `just check-commits`.
* `specs/agent-reports.md` (RPT-1 to RPT-7) marks as `deterministic` rules on the text of the skills; `test_skills.rs`
  proves them by substring matches (`check_no_narration`, `check_review_template`, `check_report_layouts`).
* The `quality` requirement kind (`benchmark:`, `metric:`) is described in `specs/README.md` and checked by
  `check_specs.rs`, but no spec uses it and no `benchmark/` directory exists.
* The `implemented` status is set by hand and enforced exactly like `accepted`.

# Approach
1. Attribution: check whether a Claude Code setting turns off the attribution at the source (`includeCoAuthoredBy` or
   its successor); if so, set it in the seeded settings and drop the hook and its test, keeping `commit_check.rs`.
2. Withdraw RPT or delete the spec, and the wording checks of `test_skills.rs` that only serve it; keep the checks of
   `test_skills.rs` on structure (front matter, cited paths).
3. Remove the `quality` kind from `specs/README.md`, `files/specs/TEMPLATE.md` and `check_specs.rs` until a benchmark
   exists.
4. Consider merging `implemented` into `accepted`.

# Specs
* `specs/agent-reports.md`: withdrawn or deleted.
* `files/specs/README.md`: the kinds and statuses change.

# Resources
* `files/.claude/hooks/block_ai_attribution.rs`, `files/.claude/skills/commit-message/commit_check.rs`
* `files/.claude/skills/test_skills.rs`, `specs/agent-reports.md`
* `files/specs/README.md`, `files/.claude/specs/check_specs.rs`, `files/specs/TEMPLATE.md`
* `seed/.claude/settings.json`

# Hints
* `harness sync` must remove the dropped managed files from the consumers: check what it does with a file that left
  `files/`.
* Consumer specs may already use `implemented`: the checker has to keep accepting it, or `sync` has to say so.
