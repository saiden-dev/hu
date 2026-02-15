#!/bin/bash
set -e

# Deploy hu to junkpile (builds remotely)
# Usage: ./deploy.sh [-f|--force]

HOST="junkpile"
REPO_PATH="/tmp/hu-build"
CARGO_ENV="source ~/.cargo/env"
INSTALL_PATH="/usr/local/bin/hu"

FORCE=false
if [ "$1" = "-f" ] || [ "$1" = "--force" ]; then
    FORCE=true
fi

echo "=== Checking if deploy needed ==="
LOCAL_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
REMOTE_VERSION=$(ssh "$HOST" "hu --version 2>/dev/null | awk '{print \$2}'" || echo "not installed")

echo "Local:  $LOCAL_VERSION"
echo "Remote: ${REMOTE_VERSION:-not installed}"

if [ "$FORCE" = false ] && [ "$LOCAL_VERSION" = "$REMOTE_VERSION" ]; then
    echo ""
    echo "=== Already up to date, skipping deploy ==="
    echo "Use -f or --force to deploy anyway"
    exit 0
fi

echo ""
echo "=== Syncing source to $HOST ==="
rsync -az --delete \
  --exclude 'target' \
  --exclude '.git' \
  . "$HOST:$REPO_PATH/"

echo ""
echo "=== Building on $HOST ==="
ssh "$HOST" "$CARGO_ENV && cd $REPO_PATH && cargo build --release"

echo ""
echo "=== Installing to $INSTALL_PATH ==="
ssh "$HOST" "sudo cp $REPO_PATH/target/release/hu $INSTALL_PATH && sudo chmod +x $INSTALL_PATH"

echo ""
echo "=== Remote Version ==="
ssh "$HOST" "hu --version"

echo ""
echo "=== Deployment Complete ==="
