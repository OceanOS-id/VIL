#!/bin/bash
# Self-contained test — no external dependencies
# Usage: cargo run --release & sleep 2 && bash test.sh
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; YELLOW="\033[0;33m"; DIM="\033[2m"; NC="\033[0m"

assert_eq() {
    local actual="$1" expected="$2" msg="${3:-assert_eq}"
    if [ "$actual" = "$expected" ]; then
        PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} $msg"
    else
        FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("$msg: expected=$expected actual=$actual")
        echo -e "  ${RED}FAIL${NC} $msg"; echo "       expected: $expected"; echo "       actual:   $actual"
    fi
}
assert_http_status() {
    local url="$1" expected="${2:-200}" msg="${3:-HTTP $expected}"
    local status; status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$url" 2>/dev/null)
    assert_eq "$status" "$expected" "$msg"
}
assert_http_status_post() {
    local url="$1" body="$2" expected="${3:-200}" msg="${4:-POST $expected}"
    local status; status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 -X POST -H "Content-Type: application/json" -d "$body" "$url" 2>/dev/null)
    assert_eq "$status" "$expected" "$msg"
}
assert_output_contains() {
    local output="$1" pattern="$2" msg="${3:-contains $2}"
    if echo "$output" | grep -q "$pattern"; then
        PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} $msg"
    else
        FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("$msg"); echo -e "  ${RED}FAIL${NC} $msg"
    fi
}
assert_contains() {
    local file="$1" pattern="$2" msg="${3:-contains $2}"
    if grep -q "$pattern" "$file" 2>/dev/null; then
        PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} $msg"
    else
        FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("$msg"); echo -e "  ${RED}FAIL${NC} $msg"
    fi
}
assert_exit_code() {
    local actual="$1" expected="${2:-0}" msg="${3:-exit code}"
    assert_eq "$actual" "$expected" "$msg"
}
skip_test() {
    SKIP_COUNT=$((SKIP_COUNT+1)); echo -e "  ${YELLOW}SKIP${NC} ${1:-skipped}"
}
print_summary() {
    local total=$((PASS_COUNT+FAIL_COUNT+SKIP_COUNT))
    echo ""; echo "────────────────────────────────────────"
    echo -e "  Total: $total  Pass: $PASS_COUNT  Fail: $FAIL_COUNT  Skip: $SKIP_COUNT"
    echo "────────────────────────────────────────"
    [ ${#FAILURES[@]} -gt 0 ] && { echo ""; for f in "${FAILURES[@]}"; do echo -e "  ${RED}✗${NC} $f"; done; echo ""; }
    [ $FAIL_COUNT -eq 0 ] && return 0 || return 1
}


PORT="${PORT:-8080}"
BASE="http://localhost:${PORT}"

echo "=== 013-basic-nats-worker (VWFD) ==="

# Health (vil_vwfd::app always has /health)
assert_http_status "$BASE/health" 200 "health endpoint"

# GET /api/nats/config
RESP=$(curl -s --max-time 10 "$BASE/api/nats/config")
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} GET /api/nats/config"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("GET /api/nats/config empty")
    echo -e "  ${RED}FAIL${NC} GET /api/nats/config empty"
fi

# GET /api/nats/jetstream
RESP=$(curl -s --max-time 10 "$BASE/api/nats/jetstream")
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} GET /api/nats/jetstream"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("GET /api/nats/jetstream empty")
    echo -e "  ${RED}FAIL${NC} GET /api/nats/jetstream empty"
fi

# GET /api/nats/kv
RESP=$(curl -s --max-time 10 "$BASE/api/nats/kv")
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} GET /api/nats/kv"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("GET /api/nats/kv empty")
    echo -e "  ${RED}FAIL${NC} GET /api/nats/kv empty"
fi

# POST /api/nats/publish
RESP=$(curl -s --max-time 10 -X POST "$BASE/api/nats/publish" -H 'Content-Type: application/json' -d '{"message":"test event"}')
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} POST /api/nats/publish"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("POST /api/nats/publish empty")
    echo -e "  ${RED}FAIL${NC} POST /api/nats/publish empty"
fi

print_summary
