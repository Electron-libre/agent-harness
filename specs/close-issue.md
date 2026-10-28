---
id: CLS
status: accepted
adrs: []
---

# Closing an issue

## Intent

`close-issue` moves a finished issue to `issues/done/`; the documents of the repository that cite it must still
point at it afterwards, whether they name it from the root or by a relative Markdown link. The agent and the user
rely on it at every close, and `--relink` repairs the references to issues closed before.

## Requirements

- **CLS-1** (deterministic) After `close-issue issues/<stem>.md`, every path `issues/<stem>.md` that starts a word (after the start of the text, a space or one of `` `"'([<{=,: ``) in a text file of the repository must read `issues/done/<stem>.md`.
- **CLS-2** (deterministic) After `close-issue issues/<stem>.md`, the target of every Markdown link of a `.md` file (`[text](target)` or `[label]: target`) that, resolved against the directory of that file, is the closed issue file must point at the issue in `issues/done/`, keeping its relative form and its `#anchor` (`../issues/foo.md#goal` becomes `../issues/done/foo.md#goal`).
- **CLS-3** (deterministic) `close-issue` must not rewrite a link target that holds a directory before `issues/` (`../issues/foo.md`, `docs/issues/foo.md`) when that target, resolved against the directory of its file, is not the closed issue file.
- **CLS-4** (deterministic) After `close-issue issues/<stem>.md`, every relative Markdown link target of the moved file that resolved to a path inside the repository from `issues/` must resolve to that same path from `issues/done/`, keeping its `#anchor`.
- **CLS-5** (deterministic) `close-issue --relink` must rewrite the references of CLS-1 and CLS-2 for every issue file in `issues/done/`.

## Scenarios

- (CLS-2) Given `docs/a.md` holding `[a](../issues/foo.md)` and `docs/adr/b.md` holding `[b](../../issues/foo.md)`, when `close-issue issues/foo.md` runs, then they read `[a](../issues/done/foo.md)` and `[b](../../issues/done/foo.md)`.
- (CLS-2) Given `issues/bar.md` holding `[foo](foo.md)`, when `close-issue issues/foo.md` runs, then it reads `[foo](done/foo.md)`.
- (CLS-3) Given `README.md` at the root holding `[x](../issues/foo.md)` (outside the repository) and `[y](docs/issues/foo.md)`, when `close-issue issues/foo.md` runs, then both are unchanged.
- (CLS-4) Given `issues/foo.md` holding `[bar](bar.md)`, `[q](queue.md#top)` and `[old](done/old.md)`, when `close-issue issues/foo.md` runs, then `issues/done/foo.md` holds `[bar](../bar.md)`, `[q](../queue.md#top)` and `[old](old.md)`.
- (CLS-1) Given `docs/a.md` holding `` `issues/foo.md` `` (a path from the root), when `close-issue issues/foo.md` runs, then it reads `` `issues/done/foo.md` ``.

## Non-goals

- Links in HTML (`<a href>`), in files other than Markdown beyond the root form of CLS-1, and URLs (`https://...`).
- Bare file names outside a Markdown link (`` `foo.md` `` in prose).
- Telling a link written as an example in code from a real one: both are rewritten.
- Checking that the other links of the repository resolve.
