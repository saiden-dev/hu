#!/usr/bin/env bash
# Version bumping script
# Usage: ./scripts/bump.sh [patch|minor]
# - No args: bump pre-release number (0.1.0-pre1 -> 0.1.0-pre2)
# - patch: bump patch version (0.1.0-pre1 -> 0.1.1-pre1, or 0.1.0 -> 0.1.1)
# - minor: bump minor version (0.1.0 -> 0.2.0, or 0.1.0-pre1 -> 0.2.0-pre1)

set -euo pipefail

current=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
echo "Current version: $current"

# Parse version components
if [[ "$current" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-pre([0-9]+))?$ ]]; then
    major="${BASH_REMATCH[1]}"
    minor="${BASH_REMATCH[2]}"
    patch="${BASH_REMATCH[3]}"
    pre="${BASH_REMATCH[5]:-}"
else
    echo "Error: Cannot parse version '$current'"
    exit 1
fi

arg="${1:-}"

if [[ "$arg" == "minor" || "$arg" == "--minor" ]]; then
    # Bump minor: 0.1.0 -> 0.2.0, 0.1.0-pre1 -> 0.2.0-pre1
    minor=$((minor + 1))
    patch=0
    if [[ -n "$pre" ]]; then
        new_version="${major}.${minor}.${patch}-pre1"
    else
        new_version="${major}.${minor}.${patch}"
    fi
elif [[ "$arg" == "patch" || "$arg" == "--patch" ]]; then
    # Bump patch: 0.1.0-pre1 -> 0.1.1-pre1, 0.1.0 -> 0.1.1
    patch=$((patch + 1))
    if [[ -n "$pre" ]]; then
        new_version="${major}.${minor}.${patch}-pre1"
    else
        new_version="${major}.${minor}.${patch}"
    fi
else
    # Bump pre-release number (default)
    if [[ -n "$pre" ]]; then
        pre=$((pre + 1))
        new_version="${major}.${minor}.${patch}-pre${pre}"
    else
        # No pre-release suffix, add -pre1
        new_version="${major}.${minor}.${patch}-pre1"
    fi
fi

echo "New version: $new_version"

# Update Cargo.toml
sed -i '' "s/^version = \"$current\"/version = \"$new_version\"/" Cargo.toml

echo "Updated Cargo.toml"
echo "Building and installing release version..."
cargo build --release
cargo install --path .

# Git commit (includes Cargo.lock updated by build), tag, and push
git add Cargo.toml Cargo.lock
git commit -m "chore: Bump version to $new_version"
git tag "v$new_version"
git push origin HEAD --tags

# Get repo URL and print link
repo_url=$(git remote get-url origin | sed 's/git@github.com:/https:\/\/github.com\//' | sed 's/\.git$//')
echo ""
echo "Done! Installed version $new_version"
echo "Release: ${repo_url}/releases/tag/v${new_version}"
