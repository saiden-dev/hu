#!/bin/bash
# Session-Start Hook: Initialize context tracking and cleanup old files
# Trigger: When a Claude Code session begins
#
# Token savings: Reduces latency, prepares index

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# Configuration
CLEANUP_DAYS="${HU_CLEANUP_DAYS:-7}"
CLAUDE_DIR="${HOME}/.claude"

# --- Cleanup old files ---

# Debug files (can grow to 400+ MB)
if [[ -d "$CLAUDE_DIR/debug" ]]; then
    find "$CLAUDE_DIR/debug" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
    echo "Cleaned debug files older than $CLEANUP_DAYS days"
fi

# Shell snapshots
if [[ -d "$CLAUDE_DIR/shell-snapshots" ]]; then
    find "$CLAUDE_DIR/shell-snapshots" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
fi

# Paste cache
if [[ -d "$CLAUDE_DIR/paste-cache" ]]; then
    find "$CLAUDE_DIR/paste-cache" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
fi

# --- Initialize context tracking ---

if command -v hu &>/dev/null; then
    # Clear any stale context from previous sessions
    hu context clear 2>/dev/null || true
    echo "Context tracking initialized"
fi

# --- Build/update code index (optional, for faster lookups) ---

# Only if in a git repo with markdown docs
if [[ -d ".git" ]] && [[ -d "doc" || -d "docs" ]]; then
    DOC_DIR="doc"
    [[ -d "docs" ]] && DOC_DIR="docs"

    if command -v hu &>/dev/null; then
        # Build index in background (don't block session start)
        INDEX_FILE="/tmp/hu-docs-index-$(basename "$(pwd)").json"
        (hu utils docs-index "$DOC_DIR" -o "$INDEX_FILE" 2>/dev/null &)
        echo "Building docs index in background: $INDEX_FILE"
    fi
fi

# --- Summary ---

echo ""
echo "Session initialized. Token-saving features active:"
echo "  - Context tracking: prevents duplicate file reads"
echo "  - Large file warnings: suggests --outline/--around"
echo "  - Grep limits: caps broad searches"
echo ""
echo "Bypass with: HU_SKIP_HOOKS=1"

exit 0
