# Fail the CI installer when the configuration is missing

# Goal
`files/.github/install_cardinal.sh` puts on the CI path the `cardinal` the project pins. When the work tree has no
`.claude/cardinal.toml`, it says the source is a local path outside the repository and exits 0: the later jobs run
without `cardinal` and the real cause is never named. Make it fail naming the missing configuration.

# Findings
* Found by the `high` review of `drop_the_old_names_of_the_consumer_files` (2026-10-29): `value` reads the file with
  `sed | head`, whose status is `head`'s, so `set -e` does not stop on a missing file; `source` is empty, the `*)`
  branch prints `source '' is a local path outside this repository: cardinal not installed` and exits 0.
* Not new: before that issue, the fallback to `.claude/harness.toml` reached the same branch when neither existed.

# Approach
1. Guard right after `config=`: `[ -f "$config" ] || { echo "no $config: cardinal not installed" >&2; exit 1; }`.
2. A test of the missing configuration in `crates/cardinal/tests/install_script.rs`, then `just sync`.

# Resources
* `files/.github/install_cardinal.sh`, `crates/cardinal/tests/install_script.rs`
* ADR 0010 (the prebuilt binary of the CI)
