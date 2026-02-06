#!/bin/bash
# Session-Start Hook: Initialize context tracking and cleanup
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

CLEANUP_DAYS="${HU_CLEANUP_DAYS:-7}"
CLAUDE_DIR="${HOME}/.claude"

[[ -d "$CLAUDE_DIR/debug" ]] && find "$CLAUDE_DIR/debug" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
echo "Cleaned debug files older than $CLEANUP_DAYS days"

[[ -d "$CLAUDE_DIR/shell-snapshots" ]] && find "$CLAUDE_DIR/shell-snapshots" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true
[[ -d "$CLAUDE_DIR/paste-cache" ]] && find "$CLAUDE_DIR/paste-cache" -type f -mtime +$CLEANUP_DAYS -delete 2>/dev/null || true

if command -v hu &>/dev/null; then
    hu context clear 2>/dev/null || true
    echo "Context tracking initialized"
fi

if [[ -d ".git" ]] && [[ -d "doc" || -d "docs" ]]; then
    DOC_DIR="doc"; [[ -d "docs" ]] && DOC_DIR="docs"
    if command -v hu &>/dev/null; then
        INDEX_FILE="/tmp/hu-docs-index-$(basename "$(pwd)").json"
        (hu utils docs-index "$DOC_DIR" -o "$INDEX_FILE" 2>/dev/null &)
        echo "Building docs index in background: $INDEX_FILE"
    fi
fi

echo -e "\nSession initialized. Token-saving features active:"
echo "  - Context tracking: prevents duplicate file reads"
echo "  - Large file warnings: suggests --outline/--around"
echo "  - Grep limits: caps broad searches"
echo -e "\nBypass with: HU_SKIP_HOOKS=1"
exit 0
