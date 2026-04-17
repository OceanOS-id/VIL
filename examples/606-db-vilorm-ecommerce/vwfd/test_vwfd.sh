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

PORT="${PORT:-8086}"
BASE="http://localhost:$PORT"

echo "=== vwfd (VWFD) ==="

assert_http_status "$BASE/health" 200 "health endpoint"

RESP=$(curl -s --max-time 10 -X POST "$BASE/api/shop/orders" -H 'Content-Type: application/json' -d '{"test":true}')
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/shop/orders"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /api/shop/orders"); echo -e "  ${RED}FAIL${NC} POST /api/shop/orders empty"; }

RESP=$(curl -s --max-time 10 -X POST "$BASE/api/shop/products" -H 'Content-Type: application/json' -d '{"test":true}')
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/shop/products"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /api/shop/products"); echo -e "  ${RED}FAIL${NC} POST /api/shop/products empty"; }

RESP=$(curl -s --max-time 10 -X DELETE "$BASE/api/shop/orders/" -H 'Content-Type: application/json' -d '{"test":true}')
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} DELETE /api/shop/orders/"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("DELETE /api/shop/orders/"); echo -e "  ${RED}FAIL${NC} DELETE /api/shop/orders/ empty"; }

RESP=$(curl -s --max-time 10 "$BASE/api/shop/products/")
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} GET /api/shop/products/"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("GET /api/shop/products/"); echo -e "  ${RED}FAIL${NC} GET /api/shop/products/ empty"; }

RESP=$(curl -s --max-time 10 "$BASE/api/shop/orders")
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} GET /api/shop/orders"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("GET /api/shop/orders"); echo -e "  ${RED}FAIL${NC} GET /api/shop/orders empty"; }

RESP=$(curl -s --max-time 10 "$BASE/api/shop/products")
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} GET /api/shop/products"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("GET /api/shop/products"); echo -e "  ${RED}FAIL${NC} GET /api/shop/products empty"; }

RESP=$(curl -s --max-time 10 "$BASE/api/shop/orders/")
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} GET /api/shop/orders/"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("GET /api/shop/orders/"); echo -e "  ${RED}FAIL${NC} GET /api/shop/orders/ empty"; }

print_summary
