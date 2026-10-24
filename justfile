# Development commands. `just` lists them; the agent harness uses the same entry points.
import 'harness.just'
set allow-duplicate-recipes

default:
    @just --list

# Format, lint (warning-free), test, then check the specs.
check: fmt clippy test check-specs

fmt:
    cargo fmt --all

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

# Install the `harness` binary to ~/.cargo/bin.
install:
    cargo install --path crates/harness --locked

# This repository is its own first consumer: check it with the binary just built.
check-harness:
    cargo run -q -p harness -- check

# Bring this repository in step with its own `files/` and `seed/`.
sync *args:
    cargo run -q -p harness -- sync {{args}}
