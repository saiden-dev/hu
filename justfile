# Default recipe
default:
    @just --list

# Run all checks
check: fmt clippy

# Format code
fmt:
    cargo fmt

# Check formatting without changes
fmt-check:
    cargo fmt --check

# Lint with clippy
clippy:
    cargo clippy -- -D warnings

# Build debug
build:
    cargo build

# Build release
release:
    cargo build --release

# Run the tool
run *ARGS:
    cargo run -- {{ARGS}}

# Install locally
install:
    cargo install --path .

# Clean build artifacts
clean:
    cargo clean

# Run tests
test:
    cargo test

# Watch for changes and rebuild
watch:
    cargo watch -x build
