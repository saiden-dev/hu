#!/bin/bash
# Pre-Grep Hook: Limit runaway searches and suggest efficient modes
# Trigger: Before Claude's Grep tool executes
#
# Token savings: 5-20x for broad searches

set -euo pipefail

# Skip if disabled
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

# Configuration
MAX_RESULTS="${HU_MAX_GREP_RESULTS:-20}"

# Parse input
INPUT=$(cat)
PATTERN=$(echo "$INPUT" | jq -r '.pattern // empty')
PATH_ARG=$(echo "$INPUT" | jq -r '.path // "."')
HEAD_LIMIT=$(echo "$INPUT" | jq -r '.head_limit // 0')

[[ -z "$PATTERN" ]] && exit 0

# Warn about overly broad patterns
BROAD_PATTERNS=(
    "^.$"           # Single character
    "^..$"          # Two characters
    "the"
    "function"
    "import"
    "return"
    "if"
    "for"
    "var"
    "let"
    "const"
)

PATTERN_LOWER=$(echo "$PATTERN" | tr '[:upper:]' '[:lower:]')
for BROAD in "${BROAD_PATTERNS[@]}"; do
    if [[ "$PATTERN_LOWER" =~ $BROAD ]]; then
        echo "Warning: Pattern '$PATTERN' may match many results."
        echo "Consider:"
        echo "  - More specific pattern"
        echo "  - Add --glob '*.rs' to limit file types"
        echo "  - Use head_limit to cap results"
        echo ""
        break
    fi
done

# Suggest --refs mode for exploratory searches
if [[ "$HEAD_LIMIT" == "0" ]] || [[ "$HEAD_LIMIT" -gt "$MAX_RESULTS" ]]; then
    echo "Tip: For exploratory searches, use output_mode: 'files_with_matches'"
    echo "     to get file paths only, then read specific files."
    echo ""
fi

# Allow grep to proceed
exit 0
