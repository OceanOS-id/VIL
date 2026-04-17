#!/bin/bash
# Identical to test_vwfd.sh but uses shared provision server
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; NC="\033[0m"

BASE="${PROVISION_URL:-http://localhost:19080}"
echo "=== 201-llm-basic-chat (provision) ==="

RESP=$(curl -s --max-time 15 -X POST "$BASE/api/chat" -H 'Content-Type: application/json' -d '{"prompt":"hello"}')
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/chat"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /api/chat empty"); echo -e "  ${RED}FAIL${NC} POST /api/chat empty"; }
