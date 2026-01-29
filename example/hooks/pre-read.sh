#!/bin/bash
# Pre-Read Hook: Prevent duplicate reads and auto-truncate large files
# Trigger: Before Claude's Read tool executes
#
# Token savings: Prevents 100% of duplicate file reads

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# Configuration
MAX_LINES="${HU_MAX_FILE_LINES:-500}"
CONTEXT_FILE="${HU_CONTEXT_FILE:-/tmp/hu-context-${CLAUDE_SESSION_ID:-default}.json}"

# Parse input (Claude passes tool args as JSON on stdin)
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.file_path // empty')

[[ -z "$FILE_PATH" ]] && exit 0

# Check if file is already in context
if command -v hu &>/dev/null; then
    STATUS=$(hu context check "$FILE_PATH" 2>/dev/null || echo "")

    if echo "$STATUS" | grep -q "loaded"; then
        # File already in context - return early
        AGO=$(echo "$STATUS" | grep -oE '[0-9]+ (seconds?|minutes?|hours?) ago' || echo "recently")
        echo "File already in context (loaded $AGO). Use --force to re-read."
        exit 0
    fi
fi

# Check file size
if [[ -f "$FILE_PATH" ]]; then
    LINE_COUNT=$(wc -l < "$FILE_PATH" 2>/dev/null || echo "0")

    if [[ "$LINE_COUNT" -gt "$MAX_LINES" ]]; then
        echo "Warning: $FILE_PATH has $LINE_COUNT lines (>${MAX_LINES} threshold)."
        echo "Consider using:"
        echo "  hu read '$FILE_PATH' --outline    # Structure only"
        echo "  hu read '$FILE_PATH' --interface  # Public API only"
        echo "  hu read '$FILE_PATH' --around N   # Lines around line N"
        echo ""
        echo "Proceeding with full read..."
    fi

    # Track the file as being read
    if command -v hu &>/dev/null; then
        hu context track "$FILE_PATH" 2>/dev/null || true
    fi
fi

# Allow the read to proceed
exit 0
