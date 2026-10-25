# Audit the instructions given to the agent to propose new skills and hooks

# Goal
The instructions a user gives an agent over time (CLAUDE.md, AGENTS.md, corrections repeated in session
transcripts, memory files) reveal practices that should be automated: a recurring multi-step procedure is a
skill, a rule the agent must never break is a hook. Today the harness grows only when the user notices such a
need. Add an audit that reads those instructions and proposes the skills and hooks worth adding, for the user
to validate.

# Findings
* The harness ships six skills (`commit-message`, `create-issue`, `issue-workflow`,
  `prioritize-issues`, `update-docs`, `write-spec`) and two hooks (`format_rust.rs`, `clippy_gate.rs`), all
  written by hand.
* No fact yet on where the repeated instructions are best found (transcripts, instruction files, memory):
  measuring this is the first step.

# Approach
Measure first: list the sources available in a consumer project and what a sample of them yields.

1. Collect the sources: instruction files (`CLAUDE.md`, `AGENTS.md`), the user's session transcripts, memory.
2. Detect candidates: instructions repeated across sessions or corrected more than once, and rules phrased as
   "never" / "always" that a hook could enforce.
3. Classify each candidate: skill (procedure with steps), hook (mechanical check on a tool call), or neither
   (a plain rule stays in the instruction file).
4. Output a report of proposals with the evidence (quotes, counts) for validation; creating them goes through
   `skill:create-issue`, nothing is generated silently.

Delivery to decide at `issue-workflow` steps 1 to 3: a skill run by the agent (no new binary), or a `harness`
subcommand. A skill keeps the shared files free of project specifics.

# Specs
* `specs/audit.md` if a `harness` subcommand is chosen; none for a skill alone.

# Resources
* `docs/architecture.md`, `files/.claude/skills/`, `files/.claude/hooks/`
* `skill:create-issue`, `skill:update-config` (hooks are declared in `settings.json`)

# Hints
* Transcripts can hold private content: the report must follow the `confidential` note of
  `.claude/harness.toml` and never be committed with raw quotes from other projects.
* The shared files hold nothing specific to one project.
