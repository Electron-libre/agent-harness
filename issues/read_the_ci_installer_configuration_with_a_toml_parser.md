# Read the CI installer configuration with a TOML parser

# Goal
`files/.github/install_cardinal.sh` reads `source` and `rev` from `.claude/cardinal.toml` with `sed` regexes in
`value()`, not as `cardinal` reads them (`[cardinal].source`, `[cardinal].rev` through a TOML parser). A valid
configuration in another form makes CI install from another source or revision than the one `cardinal` uses, or fail
with `no source read from .claude/cardinal.toml`. Make the installer read the same two values `cardinal` reads.

# Findings
* Found by the `high` review of `fail_the_ci_installer_when_the_source_is_empty` (2026-10-29). `value()` handles a
  basic or a literal string on one line, the key indented or not, and nothing else:
  * any table: a `source` or `rev` key in a table before `[cardinal]` wins (`head -n 1`);
  * an inline table `cardinal = { source = ... }`, a quoted key `"source" = ...`: not read, so the installer fails;
  * a multi-line literal `'''...'''` reads as the empty string, so it fails; a basic string with an escaped quote is
    cut at the escaped quote.
* `cardinal init` and `cardinal update` write the canonical one-line form, so only a hand-edited configuration hits
  these cases.

# Approach
1. Read the two keys with a TOML parser the runners already have, e.g.
   `python3 -c 'import tomllib, sys; ...'` (Python 3.11 or later); check first that every runner of `cardinal.yml`
   ships it (not verified).
2. Otherwise keep `sed`, but only inside `[cardinal]`: no stray key from another table, and a configuration it
   cannot read still fails with the no-source error.
In both cases keep the `no source read from` error and its tests in `crates/cardinal/tests/install_script.rs`.

# Resources
* `files/.github/install_cardinal.sh`, `crates/cardinal/tests/install_script.rs`
* `issues/fail_the_ci_installer_when_the_source_is_empty.md`
* ADR 0010 (the prebuilt binary of the CI)

# Hints
* The installer runs before `cardinal` exists, so it can't call `cardinal` to read its own configuration.
