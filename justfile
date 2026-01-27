# hu

default:
    @just --list

# Build debug
build:
    cargo build

# Run with args
run *args:
    cargo run -- {{args}}

# Run tests
test:
    cargo test

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

# Build release
release:
    cargo build --release

# Install locally
install:
    cargo install --path .

# Clean
clean:
    cargo clean

# Full check before commit
pre-commit: check test
    @echo "All checks passed"
