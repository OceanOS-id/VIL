#!/bin/bash
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; NC="\033[0m"
BASE="${PROVISION_URL:-http://localhost:19080}"
echo "=== 206-decision routing (provision) ==="
RESP=$(curl -s --max-time 10 -X POST "$BASE/api/underwrite/assess" -H 'Content-Type: application/json' -d '{"prompt":"test"}')
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/underwrite/assess returns response"
else
    FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /api/underwrite/assess empty"); echo -e "  ${RED}FAIL${NC} empty"
fi
