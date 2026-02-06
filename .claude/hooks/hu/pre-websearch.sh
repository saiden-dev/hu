#!/bin/bash
# Pre-WebSearch Hook: Log searches
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

INPUT=$(cat)
QUERY=$(echo "$INPUT" | jq -r '.query // empty')
[[ -z "$QUERY" ]] && exit 0

echo "WebSearch: \"$QUERY\""
[[ ${#QUERY} -lt 5 ]] && echo "Warning: Query is very short."
exit 0
