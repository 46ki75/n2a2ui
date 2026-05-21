# Single source of truth for what `test`, `lint`, `build`, `ci` mean in
# this repo. Other tools (CI, contributors, AI agents) invoke `just
# <recipe>`, never the underlying `cargo` command directly.

# Default recipe: show available recipes.
default:
    @just --list

# --- Formatting & lint ---------------------------------------------------

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

build:
    cargo build --workspace

# --- Hermetic tests (run on every PR) ------------------------------------

test:
    cargo test --workspace

ci: fmt-check lint test

# --- Live / approval-required tests --------------------------------------
# Hits real Notion API. Requires NOTION_API_KEY and BLOCK_ID (or .env at

# workspace root). Gated by `#[ignore]` on each live test.
test-live:
    cargo test --workspace -- --ignored

ci-live: fmt-check lint test test-live

# --- Coverage (cargo-llvm-cov) -------------------------------------------

# Instrumented hermetic test run (no report yet).
test-cov:
    cargo llvm-cov --no-report --workspace

# AI-friendly: per-file table (drop 100% files) + uncovered line numbers.
coverage: test-cov
    cargo llvm-cov report --show-missing-lines --color=always 2>&1 | grep -v " 100.00%"

# Local HTML drilldown.
coverage-html: test-cov
    cargo llvm-cov report --html --open

# CI / Codecov upload.
coverage-ci: test-cov
    cargo llvm-cov report --lcov --output-path lcov.info

# --- Live-tier coverage --------------------------------------------------

test-live-cov:
    cargo llvm-cov --no-report --workspace -- --ignored

coverage-live: test-live-cov
    cargo llvm-cov report --show-missing-lines --color=always 2>&1 | grep -v " 100.00%"
