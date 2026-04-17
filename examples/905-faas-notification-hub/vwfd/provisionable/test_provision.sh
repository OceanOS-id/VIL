#!/bin/bash
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; NC="\033[0m"
BASE="${PROVISION_URL:-http://localhost:19080}"
echo "=== 905-notification hub (provision) ==="
RESP=$(curl -s --max-time 10 -X POST "$BASE/api/notify" -H 'Content-Type: application/json' -d '{"name":"test","order_id":"ORD-1","email":"t@t.com","phone":"+628123"}')
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/notify returns response"
else
    FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /api/notify empty"); echo -e "  ${RED}FAIL${NC} empty"
fi
