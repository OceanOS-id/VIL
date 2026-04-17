#!/bin/bash
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; NC="\033[0m"

BASE="${PROVISION_URL:-http://localhost:19080}"
echo "=== 906-bench-create-order (provision) ==="

RESP=$(curl -s --max-time 10 -X POST "$BASE/api/orders" \
    -H 'Content-Type: application/json' \
    -d '{"customer":"test","items":[100,200,300],"email":"test@example.com"}')
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/orders"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /api/orders empty"); echo -e "  ${RED}FAIL${NC} empty"; }
