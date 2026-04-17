#!/bin/bash
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; NC="\033[0m"
BASE="${PROVISION_URL:-http://localhost:19080}"
echo "=== 101-pipeline-3node-transform-chain (provision) ==="
RESP=$(curl -s --max-time 10 -X POST "$BASE/transform" -H 'Content-Type: application/json' -d '{"request":"bench"}')
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /transform returns response"
else
    FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /transform empty"); echo -e "  ${RED}FAIL${NC} empty"
fi
