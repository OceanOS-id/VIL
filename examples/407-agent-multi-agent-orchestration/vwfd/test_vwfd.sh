#!/bin/bash
# Self-contained test — no external dependencies
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; YELLOW="\033[0;33m"; NC="\033[0m"
assert_http_status() {
    local url="$1" expected="${2:-200}" msg="${3:-HTTP $expected}"
    local status; status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$url" 2>/dev/null)
    if [ "$status" = "$expected" ]; then PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} $msg"
    else FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("$msg: got $status"); echo -e "  ${RED}FAIL${NC} $msg (got $status)"; fi
}
print_summary() {
    echo ""; echo "────────────────────────────────────────"
    echo -e "  Total: $((PASS_COUNT+FAIL_COUNT+SKIP_COUNT))  Pass: $PASS_COUNT  Fail: $FAIL_COUNT  Skip: $SKIP_COUNT"
    echo "────────────────────────────────────────"
    [ ${#FAILURES[@]} -gt 0 ] && { for f in "${FAILURES[@]}"; do echo -e "  ${RED}✗${NC} $f"; done; }
}

PORT="${PORT:-8080}"
BASE="http://localhost:$PORT"

echo "=== 407-agent-multi-agent-orchestration (VWFD) ==="

assert_http_status "$BASE/health" 200 "health endpoint"

RESP=$(curl -s --max-time 15 -X POST "$BASE/api/support/ask" -H 'Content-Type: application/json' -d '{"prompt":"What is VIL?"}')
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/support/ask"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /api/support/ask empty"); echo -e "  ${RED}FAIL${NC} POST /api/support/ask empty"; }

print_summary
