# hu

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

# CI tasks
ci-fmt:
    cargo fmt

ci-lint-fix:
    cargo clippy --fix --allow-dirty --allow-staged -- -D warnings

ci-lint-check:
    cargo clippy -- -D warnings

ci-test:
    cargo test --verbose

ci-build target:
    cargo build --release --target {{target}}

ci-install-cross-deps target:
    ./scripts/install-cross-deps.sh {{target}}

ci-package target version:
    ./scripts/package.sh {{target}} {{version}}

ci-is-prerelease tag:
    @./scripts/is-prerelease.sh {{tag}}

ci-verify-version tag:
    ./scripts/verify-version.sh {{tag}}

ci-publish:
    cargo publish --token $CRATES_API_KEY

# Full release prep
dist: lint test release
    @echo "Release ready in target/release/"
    @ls -lh target/release/hu

# Watch for changes and rebuild
watch:
    cargo watch -x build

# GitHub Actions
# Clear failed workflow runs
gh-clear:
    gh run list --status failure --limit 1000 --json databaseId -q '.[].databaseId' | xargs -I{} gh run delete {}

# Clear all workflow runs
gh-clear-all:
    gh run list --limit 1000 --json databaseId -q '.[].databaseId' | xargs -I{} gh run delete {}
