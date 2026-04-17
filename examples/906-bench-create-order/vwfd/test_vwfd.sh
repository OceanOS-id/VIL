#!/bin/bash
# Self-contained test
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; YELLOW="\033[0;33m"; NC="\033[0m"
PORT=${PORT:-8080}; BASE="http://localhost:$PORT"
echo "=== 906-bench-create-order (VWFD) ==="
# Health
STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$BASE/health" 2>/dev/null)
[ "$STATUS" = "200" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} health"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); echo -e "  ${RED}FAIL${NC} health ($STATUS)"; }
RESP=$(curl -s --max-time 10 -X POST "$BASE/api/orders" -H 'Content-Type: application/json' -d '{"test":true}')
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/orders"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); echo -e "  ${RED}FAIL${NC} POST /api/orders empty"; }
echo ""; echo "  Pass: $PASS_COUNT  Fail: $FAIL_COUNT  Skip: $SKIP_COUNT"
