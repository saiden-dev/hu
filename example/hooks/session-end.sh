#!/bin/bash
# Session-End Hook: Cleanup context tracking
# Trigger: When a Claude Code session ends
#
# Keeps context files from accumulating

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# --- Clear context tracking ---

if command -v hu &>/dev/null; then
    # Get summary before clearing (for logging)
    SUMMARY=$(hu context summary 2>/dev/null || echo "")

    if [[ -n "$SUMMARY" ]] && ! echo "$SUMMARY" | grep -q "No files"; then
        echo "Session context summary:"
        echo "$SUMMARY"
        echo ""
    fi

    # Clear the context
    hu context clear 2>/dev/null || true
fi

# --- Remove temp index files ---

INDEX_FILE="/tmp/hu-docs-index-$(basename "$(pwd)").json"
[[ -f "$INDEX_FILE" ]] && rm -f "$INDEX_FILE"

echo "Session cleanup complete"
exit 0
