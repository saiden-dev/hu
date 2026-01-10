#!/usr/bin/env bash
set -euo pipefail

# Install cross-compilation dependencies
# Usage: install-cross-deps.sh <target>

TARGET="$1"

case "$TARGET" in
    aarch64-unknown-linux-gnu)
        sudo dpkg --add-architecture arm64
        sudo sed -i 's/^deb /deb [arch=amd64] /' /etc/apt/sources.list
        echo "deb [arch=arm64] http://ports.ubuntu.com/ $(lsb_release -cs) main restricted universe multiverse" | sudo tee -a /etc/apt/sources.list
        echo "deb [arch=arm64] http://ports.ubuntu.com/ $(lsb_release -cs)-updates main restricted universe multiverse" | sudo tee -a /etc/apt/sources.list
        sudo apt-get update
        sudo apt-get install -y gcc-aarch64-linux-gnu libssl-dev:arm64 pkg-config
        ;;
    *)
        # No extra dependencies needed
        ;;
esac
