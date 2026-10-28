# Development commands. `just` lists them; the agent harness uses the same entry points.
import 'harness.just'
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

# Install the `harness` binary to ~/.cargo/bin.
install:
    cargo install --path crates/cardinal --locked

# This repository is its own first consumer: check it with the binary just built.
check-harness:
    cargo run -q -p harness -- check

# The spec checker of the working tree, whatever `harness` is installed (CI installs none for this repository).
check-specs *args:
    @cargo run -q -p harness -- run check-specs {{args}}

# Bring this repository in step with its own `files/` and `seed/`.
sync *args:
    cargo run -q -p harness -- sync {{args}}
