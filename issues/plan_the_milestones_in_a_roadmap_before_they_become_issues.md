# Plan the milestones in a roadmap before they become issues

# Goal
What is planned but not yet an issue has no home in the repository: it stays in the user's head or in the agent's
memory, out of reach of the skills that rank and write the issues. Give each project a roadmap document that groups
the planned work into milestones, each milestone turning into issues when it is shaped, so the queue follows a macro plan
instead of being the plan.

# Findings
* The `roadmap` key of `.claude/cardinal.toml` exists and three skills read it (`issue-workflow` step 1,
  `create-issue` step 2, `prioritize-issues` step 2), but nothing writes it: no template, no seeded file, no skill.
* This repository points it at its own queue (`roadmap = ["issues/queue.md"]`); the seeded default is
  `docs/TODO.md`, a file that is never seeded.
* No link exists between a planned theme and the issues it produced: an issue can't say what it serves, and nothing
  says a theme is done.
* Planned work that is not an issue lands in the agent's memory today (e.g. an article planned after a series of
  issues), invisible to the other sessions and to the user's review.

# Approach
Decided with the user before the issue was written: the word "milestone" (not "epic", too connoted), one file,
milestones ordered by the user, no mechanical check yet. A milestone carries no date unless the project gives it one.

1. Seed a `docs/roadmap.md` (copied once, owned by the project) and point the seeded `roadmap` key at it. One section
   per milestone, in the order the user wants: goal, why, state (`idea`, `shaped`, `in issues`, `done`), the issues it
   produced (links).
2. `skill:create-issue` attaches the new issue to a milestone (adds the link under it and names the milestone in the issue), or
   says it has none.
3. `skill:prioritize-issues` ranks the queue along the order of the milestones; it never reorders the milestones themselves
   (the user's call).
4. Turning a `shaped` milestone into issues sized by the limit of `skill:issue-workflow` step 1: a step of
   `skill:create-issue` or a skill of its own, to choose at step 1.
5. Optional: `just close-issue` sets the milestone `done` when its last issue closes.

Out of scope for now: a mechanical check of the milestone links (an issue with no milestone, a `done` milestone with an open issue)
in `just check-issue`; reconsider once the roadmap is in use and drift is measured.

This repository adopts it too: write its own `docs/roadmap.md` with the current themes and point its `roadmap` key at
it (keeping `issues/queue.md` if the skills still need it there).

# Resources
* `.claude/cardinal.toml` and `seed/.claude/cardinal.toml` (the `roadmap` key), `docs/architecture.md` (seeded versus
  managed content).
* `files/.claude/skills/create-issue/SKILL.md`, `files/.claude/skills/prioritize-issues/SKILL.md`,
  `files/.claude/skills/issue-workflow/SKILL.md`.
* `specs/delivered-quality.md` (QUA-14: the `roadmap` files never count as doc lines).
* `docs/adr/0017-the-source-declares-its-migrations.md`.

# Hints
* Changing the seeded default of `roadmap` changes a seeded text: check whether ADR 0017 asks for a migration
  (existing consumers keep their own value).
* The roadmap is a document of the project, never managed content: `cardinal sync` must not rewrite it.
* `delivered-quality` excludes the `roadmap` files from the doc lines: keep the new file listed in the key so editing
  the roadmap is not counted as documentation work.
