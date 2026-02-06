#!/bin/bash
# Pre-Read Hook: Prevent duplicate reads and auto-truncate large files
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

MAX_LINES="${HU_MAX_FILE_LINES:-500}"
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.file_path // empty')
[[ -z "$FILE_PATH" ]] && exit 0

if command -v hu &>/dev/null; then
    STATUS=$(hu context check "$FILE_PATH" 2>/dev/null || echo "")
    if echo "$STATUS" | grep -q "loaded"; then
        AGO=$(echo "$STATUS" | grep -oE '[0-9]+ (seconds?|minutes?|hours?) ago' || echo "recently")
        echo "File already in context (loaded $AGO). Use --force to re-read."
        exit 0
    fi
fi

if [[ -f "$FILE_PATH" ]]; then
    LINE_COUNT=$(wc -l < "$FILE_PATH" 2>/dev/null || echo "0")
    if [[ "$LINE_COUNT" -gt "$MAX_LINES" ]]; then
        echo "Warning: $FILE_PATH has $LINE_COUNT lines (>${MAX_LINES} threshold)."
        echo "Consider: hu read '$FILE_PATH' --outline | --interface | --around N"
    fi
    command -v hu &>/dev/null && hu context track "$FILE_PATH" 2>/dev/null || true
fi
exit 0
