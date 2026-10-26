# Detect the breaches of the harness rules in the transcripts

# Goal
Know whether the agent follows the rules the harness states (AGENTS.md, CLAUDE.md, skills, ADRs), and which
rules are broken often enough to deserve a hook or a rewording. Only the breaches a program can see are in scope.

# Findings
* 2026-10-26: the review gate refusals are visible in the transcripts (for instance "the last code review predates
  the last commit", "only the user waives the code review"); a refusal followed by another way to the same commit
  is what ADR 0005 calls a bypass.
* Skill calls named `files:commit-message`, `files:create-issue`, `files:write-spec`, `files:update-docs` appear
  in this repository's transcripts: the agent picked the source copy under `files/` instead of the synced one.
* Rules a program can check from a transcript: no AI attribution trailer in commit messages, Conventional Commits
  format, conversation in the `user_language` of `.claude/harness.toml`, no edit of a managed copy in a consumer,
  one worktree per issue (no issue work on the default branch), the single validation stop of `issue-workflow`
  before any test or code, no `--no-verify`.

# Approach
1. A list of detectors, one per rule, each reading the tool calls and results of a session and reporting the
   breaches with the session and the line; start with the cheapest (commit commands, edited paths, branch).
2. Count, per rule and per harness revision, the breaches over the sessions where the rule applied, so a rule that
   rarely applies is not read as respected.
3. For the rules that need judgment (the shape of the reports to the user), sample sessions and leave the verdict
   to a later LLM judge: out of scope here.
4. Feed the result to `skill:audit-instructions`: a rule broken often is a hook candidate.

# Specs
* `specs/evaluation.md` (from `extract_session_metrics_from_the_transcripts`), new requirements for the detectors.

# Resources
* `files/AGENTS.md`, `CLAUDE.md`, `.claude/harness.toml`
* `docs/adr/0005-the-gates-catch-mistakes-not-bypasses.md`, `docs/adr/0003-one-worktree-per-issue.md`
* `files/.claude/hooks/require_code_review.rs` (its refusal messages)
* `issues/done/extract_session_metrics_from_the_transcripts.md`

# Hints
* A breach already blocked by a hook is not a breach of the agent's behaviour but a gate doing its job; report
  both apart.
* The `files:` skill calls may be a harness defect (skills under `files/` being discovered) rather than an agent
  mistake: check before counting them.
