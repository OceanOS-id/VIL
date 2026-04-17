#!/bin/bash
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; NC="\033[0m"
BASE="${PROVISION_URL:-http://localhost:19080}"
echo "=== 904-financial analytics (provision) ==="
RESP=$(curl -s --max-time 10 -X POST "$BASE/api/finance/analyze" -H 'Content-Type: application/json' -d '{"birth_date":"1990-01-15","transactions":[100,200,300]}')
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/finance/analyze returns response"
else
    FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /api/finance/analyze empty"); echo -e "  ${RED}FAIL${NC} empty"
fi
