#!/bin/bash
# Pre-WebFetch Hook: Log URL fetches
set -euo pipefail
[[ "${HU_SKIP_HOOKS:-}" == "1" ]] && exit 0

INPUT=$(cat)
URL=$(echo "$INPUT" | jq -r '.url // empty')
[[ -z "$URL" ]] && exit 0

echo "WebFetch: $URL"
[[ "$URL" == http://* ]] && echo "Warning: Using HTTP instead of HTTPS"
exit 0
