# hu

default:
    @just --list

# Build debug
build:
    cargo build

# Run with args
run *args:
    cargo run -- {{args}}

# Run unit tests only
unit:
    cargo test

# Run tests with coverage
coverage:
    cargo tarpaulin --out Html

# Run all checks and tests (fix + check + tests + coverage)
test: fix check unit coverage
    @echo "All checks passed"

# Format code
fmt:
    cargo fmt

# Lint with clippy
clippy:
    cargo clippy -- -D warnings

# Run all checks (fmt + clippy)
check:
    cargo fmt --check
    cargo clippy -- -D warnings

# Fix all formatting and lints
fix:
    cargo fmt
    cargo clippy --fix --allow-dirty --allow-staged -- -D warnings

# Build release
release:
    cargo build --release

# Install locally
install:
    cargo install --path .

# Clean
clean:
    cargo clean

# Alias for test (full check before commit)
pre-commit: test
