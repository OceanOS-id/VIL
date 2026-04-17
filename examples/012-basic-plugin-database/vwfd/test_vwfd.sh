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

echo "=== 012-basic-plugin-database (VWFD) ==="

# Health (vil_vwfd::app always has /health)
assert_http_status "$BASE/health" 200 "health endpoint"

# POST /tasks
RESP=$(curl -s --max-time 10 -X POST "$BASE/tasks" -H 'Content-Type: application/json' -d '{"title":"Test Task","description":"test"}')
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} POST /tasks"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("POST /tasks empty")
    echo -e "  ${RED}FAIL${NC} POST /tasks empty"
fi

# GET /config
RESP=$(curl -s --max-time 10 "$BASE/config")
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} GET /config"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("GET /config empty")
    echo -e "  ${RED}FAIL${NC} GET /config empty"
fi

# GET /products
RESP=$(curl -s --max-time 10 "$BASE/products")
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} GET /products"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("GET /products empty")
    echo -e "  ${RED}FAIL${NC} GET /products empty"
fi

# GET /plugins
RESP=$(curl -s --max-time 10 "$BASE/plugins")
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} GET /plugins"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("GET /plugins empty")
    echo -e "  ${RED}FAIL${NC} GET /plugins empty"
fi

# GET /tasks
RESP=$(curl -s --max-time 10 "$BASE/tasks")
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} GET /tasks"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("GET /tasks empty")
    echo -e "  ${RED}FAIL${NC} GET /tasks empty"
fi

# GET /pool-stats
RESP=$(curl -s --max-time 10 "$BASE/pool-stats")
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} GET /pool-stats"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("GET /pool-stats empty")
    echo -e "  ${RED}FAIL${NC} GET /pool-stats empty"
fi

# GET /redis-ping
RESP=$(curl -s --max-time 10 "$BASE/redis-ping")
if [ -n "$RESP" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} GET /redis-ping"
else
    FAIL_COUNT=$((FAIL_COUNT + 1)); FAILURES+=("GET /redis-ping empty")
    echo -e "  ${RED}FAIL${NC} GET /redis-ping empty"
fi

print_summary
