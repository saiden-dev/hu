#!/bin/bash
# Pre-Grep Hook: Warn on broad patterns
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

INPUT=$(cat)
PATTERN=$(echo "$INPUT" | jq -r '.pattern // empty')
[[ -z "$PATTERN" ]] && exit 0

BROAD_PATTERNS=("^.$" "^..$" "the" "function" "import" "return" "if" "for" "var" "let" "const")
PATTERN_LOWER=$(echo "$PATTERN" | tr '[:upper:]' '[:lower:]')
for BROAD in "${BROAD_PATTERNS[@]}"; do
    if [[ "$PATTERN_LOWER" =~ $BROAD ]]; then
        echo "Warning: Pattern '$PATTERN' may match many results."
        break
    fi
done
exit 0
