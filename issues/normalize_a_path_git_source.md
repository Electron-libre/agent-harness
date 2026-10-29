# Normalize a path git source

# Goal
SYN-49 makes a path that ends with `.git` (`/srv/x.git`, `../x.git`, `/x/.git`) a git source to clone, but the
code that clones, caches and builds it treats the string as given. A relative one resolves against the process
cwd instead of the project root, two projects with the same relative string share one clone, and `cargo install
--git` refuses a path. Resolve such a source once against the project root and use the same form everywhere, the CI
installer included.

# Findings
* Found by the `high` review of `restore_the_git_source_detection`; none observed in use. All predate it: the rule
  it restored is the one before `e5352ff`.
* `fetch` (`crates/cardinal/src/lib.rs`, `git clone <source>`) runs in the process cwd, while a local directory
  source is read at `root.join(source)`. `cardinal run` walks up to the project root, so a hook run from a
  subdirectory clones `../x.git` from the wrong place.
* `clone_dir` hashes the raw source string: two projects whose `../x.git` names different repositories share one
  cache clone, and the second fetches the first's repository with no error.
* `cargo_url` (`crates/cardinal/src/run.rs`) passes a path through: `cargo install --git /x/.git` fails with
  `invalid url ... relative URL without a base`, so the build fallback of `cardinal run` and the install command
  `launcher::install` prints fail for such a source.
* `files/.github/install_cardinal.sh` counts only `*://*|git@*` as git: a bare `/srv/cardinal.git` source falls in
  the local branch, finds no crate and exits 0, so CI runs without `cardinal`.

# Approach
1. Turn a path-like git source into an absolute `file://` URL resolved against the project root at the one place
   the source is read; the clone cwd, the cache key and `cargo_url` then follow.
2. Align `install_cardinal.sh` with SYN-49 (`*.git`), resolving the path the same way.
3. Test `cardinal run` and `sync` with a relative `.git` source run from a subdirectory.

# Specs
* `specs/sync.md`: SYN-49 amended, or a new requirement on how a path git source is resolved.

# Resources
* `crates/cardinal/src/lib.rs` (`is_git_url`, `clone_dir`, `fetch`), `run.rs` (`cargo_url`, `build`), `launcher.rs`
* `files/.github/install_cardinal.sh`
* `issues/done/restore_the_git_source_detection.md`

# Tracking

Plan: validated

## Understanding
A path git source (`../x.git`, `/x/.git`) becomes one absolute, percent-encoded `file://` URL resolved against the
project root, used to clone, as the cache key and by `cargo install --git`, in the binary and in the CI installer.
Out of scope: scp-like `host:path` (kept as written), any change to local directory sources or to what SYN-49 counts as git.

## Plan
1. [x] Resolve a path git source in `cardinal`: `source_and_rev` and `init` resolve it (`lib.rs`), so `fetch`,
   `clone_dir`, `cargo_url` and `launcher::install` see the URL; verified by binary tests (`sync` of two projects
   with `../x.git` next to different repositories, `cardinal run` from a subdirectory, the install command printed);
   makes true SYN-50. ~250 lines.
2. [ ] Align `files/.github/install_cardinal.sh`: `*.git` is git (SYN-49), a path one resolved to the same URL;
   verified by a Rust test that runs the script with a stub `cargo` and checks the `--git` URL it gets; makes true
   SYN-50. ~150 lines.

## Decisions
- Mode: autonomous
- Base commit: 9cf4dd7af648b815b82b37633704549853485be8
- Dot segments removed lexically (1 a): the URL does not depend on the repository existing or on symlinks.

## Deliverable 1

- done: `resolved` (`lib.rs`) turns a path git source into its absolute `file://` URL from the project root, applied
  by `source_and_rev` and `init`; `docs/architecture.md` names both forms and the `cardinal` cache directory.
- red: SYN-50: `fatal: repository '../x.git' does not exist` (`init --root`, and `cardinal run` from a subdirectory);
  the unit test did not compile (`resolved` missing).
- checks: `just check` ok (fmt, clippy, tests, specs); docs updated.
- Code review: no findings
- not verified: a source path that is not valid UTF-8 (read lossily).
