# EKS Shell

default:
    @just --list

# Development
build:
    cargo build

run *args:
    cargo run -- {{args}}

test:
    cargo test

lint:
    cargo clippy -- -D warnings
    cargo fmt --check

fmt:
    cargo fmt

# Alias for consistency
check: lint

clippy:
    cargo clippy -- -D warnings

# Release
release:
    cargo build --release

install:
    cargo install --path .

# Version bumping
# Usage: just bump [patch|minor]
# - No args: bump pre-release number (0.1.0-pre1 -> 0.1.0-pre2)
# - patch: bump patch version (0.1.0-pre1 -> 0.1.1-pre1, or 0.1.0 -> 0.1.1)
# - minor: bump minor version (0.1.0 -> 0.2.0, or 0.1.0-pre1 -> 0.2.0-pre1)
bump *args:
    ./scripts/bump.sh {{args}}

# Clean
clean:
    cargo clean

# Full release prep
dist: lint test release
    @echo "Release ready in target/release/"
    @ls -lh target/release/eks-shell

# Watch for changes and rebuild
watch:
    cargo watch -x build
