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
