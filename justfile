# Development commands. `just` lists them; the agent harness uses the same entry points.
import 'cardinal.just'
set allow-duplicate-recipes

default:
    @just --list

# Format, lint (warning-free), test, then check the specs; prints nothing on success, since the agent reads all of it.
# Strict: this repository ships the managed tests, so their citations must resolve to a spec here (consumers stay
# lenient).
check: fmt clippy test
    @just check-specs --strict

fmt:
    @cargo fmt --all

clippy:
    @cargo clippy -q --workspace --all-targets -- -D warnings

# The output of the tests, only when they fail.
test:
    #!/usr/bin/env sh
    out=$(cargo test -q --workspace 2>&1) || { printf '%s\n' "$out"; exit 1; }

# Install the `cardinal` binary to ~/.cargo/bin (`--force`: over the installed one, even of the same version).
install:
    cargo install --force --path crates/cardinal --locked

# This repository is its own first consumer: check it with the binary just built.
check-cardinal:
    cargo run -q -p cardinal -- check

# The spec checker of the working tree, whatever `cardinal` is installed (CI installs none for this repository).
check-specs *args:
    @cargo run -q -p cardinal -- run check-specs {{args}}

# Write `version` into the crate and its lock (the `release_version_command` of `just release`).
set-version version:
    sed -i.bak '/^\[package\]/,/^\[/s/^version = ".*"/version = "{{version}}"/' crates/cardinal/Cargo.toml && rm crates/cardinal/Cargo.toml.bak
    cargo update --workspace --offline -q

# Bring this repository in step with its own `files/` and `seed/`.
sync *args:
    cargo run -q -p cardinal -- sync {{args}}
