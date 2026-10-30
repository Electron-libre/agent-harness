# Keep the user and architecture docs in step with the specs

# Goal

In the consumers, the specs grow fast while the user docs and the architecture document barely move. The docs step
of `issue-workflow` does run, but it only patches a line or two per deliverable: nothing routes a change of behaviour
to the page that describes the capability, nothing reads the docs over a whole issue, and nothing measures it.
Expected outcome: a change of behaviour updates its spec **and** the doc page and architecture document that describe
it, and `delivered-quality` shows when it does not.

# Findings (2026-10-30, two consumer projects, since their specs started on 2026-10-24)

* The docs step runs: 83 % and 85 % of the `feat`/`fix` commits touch `docs/` (outside ADRs); only 3 commits in each
  project change a spec and no doc, README or `CLAUDE.md`.
* `skill:update-docs` is invoked in 24 of 46 and 39 of 67 deliverables (counted as `commit-message` calls), about
  half; the other deliverables edit the docs inline or not at all.
* Specs: +756 / −0 and +918 / −0 lines. Docs outside ADRs: +151 / −97 in one project, about +175 net in the other
  (leaving out a rename of the doc folder).
* Architecture document: 12 changed lines over 123 `feat`/`fix` commits in one project, 82 in the other.
* The `docs` row of the 3.6 summaries mostly names real pages, but as surgical edits.
* The generic row of `update-docs` for "Behaviour of a capability that has a spec" sends to `specs/<capability>.md`
  only. In one project the `update-docs-map.md` has no other row for a change of behaviour, so a behaviour change
  goes to the spec alone.
* `issue-workflow` step 3.4 touches the `architecture` document only "when the flow changed", and only its diagrams.
  The final `high` review reads `base..HEAD` for code; no docs pass covers the whole issue.
* `delivered-quality` excludes `docs/` from the files of an issue (QUA-4) and has no docs column.

# Approach

Candidate solutions, can be combined:

1. `update-docs`, generic row: a behaviour change of a capability with a spec also updates the `docs/` page that
   describes the capability, and the `architecture` document of `.claude/cardinal.toml` when a boundary, a flow or
   a component changes (not only its diagrams).
2. `issue-workflow`, after the last deliverable: run `skill:update-docs` once on `base..HEAD`, next to the `high`
   review. It reads the touched user pages and the architecture document whole, and consolidates them instead of
   patching them. Its edits go in the fix deliverable of that review.
3. `delivered-quality`: a column counting the commits of an issue that change a file under `specs/` and no
   documentation file (`docs/` outside `docs/adr/`, `README.md`, `CLAUDE.md`), so the commits of 1 and 2 can carry
   `Expected-effect: <column> down` and `skill:evaluate-harness` can judge them.

Measure first for 3: check that the column separates the two consumers' history before relying on it.

# Specs

* `specs/delivered-quality.md`: a new requirement (`QUA-14`) for the column of approach 3, and QUA-1 amended for its
  place in the header.

# Resources

* `files/.claude/skills/update-docs/SKILL.md` (step 2 table, step 4)
* `files/.claude/skills/issue-workflow/SKILL.md` (steps 3.4 and 3.5, the final `high` review)
* `specs/delivered-quality.md`, `specs/cardinal-verdicts.md`
* `docs/architecture.md` (the readers of `cardinal-report`)

# Hints

* Approaches 1 and 2 change `files/`, so every consumer: run `just sync` and give the commits an `Expected-effect`.
* Watch the fixed context: approach 2 adds a step to the longest skill; keep it to one line in `SKILL.md` and put
  the detail in `details.md` if needed.
* A pure refactor or a test seam still needs no doc (the "n/a" rows met in the summaries were justified).
