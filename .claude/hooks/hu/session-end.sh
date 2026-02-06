#!/bin/bash
# Session-End Hook: Cleanup context tracking
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

if command -v hu &>/dev/null; then
    SUMMARY=$(hu context summary 2>/dev/null || echo "")
    if [[ -n "$SUMMARY" ]] && ! echo "$SUMMARY" | grep -q "No files"; then
        echo "Session context summary:"
        echo "$SUMMARY"
    fi
    hu context clear 2>/dev/null || true
fi

INDEX_FILE="/tmp/hu-docs-index-$(basename "$(pwd)").json"
[[ -f "$INDEX_FILE" ]] && rm -f "$INDEX_FILE"
echo "Session cleanup complete"
exit 0
