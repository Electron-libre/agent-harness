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

# Tracking
1. [x] `harness check` warns of a missing hook and of an attribution left on (SYN-25, SYN-26, SYN-24 scenario).
2. [ ] Attribution off in the seeded settings; drop `block_ai_attribution.rs` and its test.
3. [ ] Delete `specs/agent-reports.md` and the RPT checks of `test_skills.rs`.
4. [ ] Drop the `quality` kind; merge `implemented` into `accepted` (still accepted, with a warning).

## Decisions
* Mode: autonomous.
* Attribution: `"attribution": {"commit": "", "pr": "", "sessionUrl": false}` (the `false` form needs Claude Code
  v2.1.281).
* RPT: the spec is deleted, not withdrawn; RPT-7 was already moot (the native hook reads no report marker).
* `implemented`: the checker keeps accepting it with a warning, so consumer specs do not break.
* SYN-25 and SYN-26 apply to any existing `.claude/settings.json`; the unreadable-settings warning stays single and
  no longer depends on the review hook being managed.

## Deliverable 1
* done: `check` warns of a hook the settings run and the project lacks, and of an attribution left on; the
  settings warnings moved to `crates/harness/src/settings.rs`, read once, the unreadable-settings warning no longer
  needs the review hook.

| check | result |
|---|---|
| tests | `just check`, `just test-harness` pass |
| lint | clippy clean |
| specs | SYN-25, SYN-26 traced; `sync.md` back to `implemented` |
| docs | README, `docs/architecture.md`, `main.rs` help |
| code review | see below |

Code review:
* the attribution warning fires in this repo and every seeded project until deliverable 2: expected, deliverable 2
  sets it in both settings.
* a `.claude/hooks/` path outside the project (`~/.claude/hooks/x`) was reported: fixed, only a bare, `./` or
  `$CLAUDE_PROJECT_DIR/` prefix counts.
* a path followed by `)`, `>` and the like kept those characters: fixed, the file name stops at the first character
  that is not alphanumeric, `_`, `-` or `.`.
* a bare `.claude/hooks/` or a glob was reported: fixed, skipped.
* `attribution: false` is said undocumented: not fixed, the Claude Code settings reference documents it (v2.1.281);
  the deprecated `includeCoAuthoredBy: false` still gets the warning on purpose, SYN-26 asks for `attribution`.
* one parse feeding the three checks behind a `gated` flag: not changed, a single read is what SYN-24's scenario asks
  (one warning for unreadable settings).
* a line over 120 characters in `docs/architecture.md`: fixed.

not verified: —
decisions needed: —
