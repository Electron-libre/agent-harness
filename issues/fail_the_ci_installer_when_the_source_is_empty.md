# Fail the CI installer when the source is empty

# Goal
`files/.github/install_cardinal.sh` now fails when `.claude/cardinal.toml` is missing, but a configuration it reads
no `source` from still takes the `*)` local-path branch, prints `source '' is a local path outside this repository:
cardinal not installed` and exits 0: the later jobs run without `cardinal` and the real cause is never named. Make an
empty source fail, naming the key and the file.

# Findings
* Found by the `high` review of `fail_the_ci_installer_when_the_configuration_is_missing` (2026-10-29), three ways to
  an empty `source` past the `[ -f "$config" ]` guard:
  * no `source` key in the configuration;
  * a TOML literal string, `source = '...'`: `value()` only matches double-quoted basic strings;
  * an unreadable configuration: `sed` fails, but `value()` returns the status of `head`, so `set -e` does not stop.
* The `*)` branch exits 0 on purpose for a real local path outside the repository; only the empty source is wrong.

# Approach
1. Guard right after `source=`: `[ -n "$source" ] || { echo "no source in $config: cardinal not installed" >&2; exit 1; }`
   (covers the three cases; `[ -r ]` alone would cover only the last).
2. Read single-quoted literal strings in `value()` too, so a valid `source = '...'` installs.
3. Tests in `crates/cardinal/tests/install_script.rs` (no `source` key; a single-quoted source), then `just sync`.

# Resources
* `files/.github/install_cardinal.sh`, `crates/cardinal/tests/install_script.rs`
* `issues/done/fail_the_ci_installer_when_the_configuration_is_missing.md`
* ADR 0010 (the prebuilt binary of the CI)
