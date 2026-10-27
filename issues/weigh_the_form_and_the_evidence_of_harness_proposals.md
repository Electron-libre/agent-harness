# Weigh the form and the evidence of harness proposals

# Goal
`skill:audit-instructions` classifies a candidate as a skill, a hook or neither, with no notion of what each form
costs the context, and neither it nor `skill:evaluate-harness` states when the evidence is too thin to act. Give them
a cost-ordered ladder of forms and explicit evidence rules, so the harness grows only where the transcripts argue
for it.

# Findings
* `files/.claude/skills/audit-instructions/SKILL.md` step 3: the classes are skill (a procedure), hook (a mechanical
  check) and neither (a rule that stays in the instruction file); nothing says that always-on text is paid on every
  turn while a skill body is loaded on demand.
* The same skill has no "propose nothing" outcome and no threshold below which a correction is noise.
* `google-research/harness2` (`harness2/core/prompts.py`) states, for an agent editing a harness: prefer an
  on-demand skill, then a script, then a subagent, then a hook, and always-on text last ("if you add always-on text,
  cut something else to pay for it"); the hook shapes it names are a targeted hint after a recurring error
  signature, trimming noisy tool output and checking a write right after it; "KEEP_BASE is a real answer";
  a defect seen in 1 of k runs is noise; each decision is written `kept|revised|dropped -> <edit> -> <evidence,
  with counts>`.

# Approach
1. In `audit-instructions` step 3, order the classes by context cost (skill, script or `harness` tool, hook,
   always-on rule last) and require that a proposed always-on rule names what it replaces or trims.
2. List the hook shapes beyond blocking (a hint after a recurring error, trimmed output, a check after a write).
3. Add the evidence rules to both skills: proposing nothing is a valid outcome; a correction seen in one session
   only is not a candidate; each row cites its counts.
4. Check the fixed context the change adds (`harness run fixed-context`) and pay for it in the same skills.

# Resources
* `files/.claude/skills/audit-instructions/SKILL.md`, `files/.claude/skills/evaluate-harness/SKILL.md`
* `crates/harness/tests/skills.rs` (the tests of the shared skills)
* `https://github.com/google-research/harness2`, `harness2/core/prompts.py` (`_SURFACE_CODEX`, `aggregate`,
  `experience`)

# Hints
* Edit `files/`, then `just sync`: the copies in `.claude/` are managed.
* The skills are already long; the change should trim as much as it adds.
