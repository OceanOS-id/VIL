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


echo "=== 047-basic-custom-error-stack ==="

PORT=${PORT:-8080}
BASE="http://127.0.0.1:${PORT}"

# Test: GET /api/banking/accounts
assert_http_status "${BASE}/api/banking/accounts" "200" \
    "GET /api/banking/accounts returns HTTP 200"

# Test: POST /api/banking/transfer — valid transfer
TRANSFER_PAYLOAD=$(cat "$SCRIPT_DIR/payloads/transfer-valid.json")
assert_http_status_post "${BASE}/api/banking/transfer" "$TRANSFER_PAYLOAD" "200" \
    "POST /api/banking/transfer valid transfer returns HTTP 200"

# Test: POST /api/banking/transfer — transaction limit exceeded
# ACC-1001 daily_limit=100_000_00, amount 999999999 > limit → TRANSACTION_LIMIT_EXCEEDED
LIMIT_PAYLOAD='{"from_account":"ACC-1001","to_account":"ACC-1002","amount_cents":999999999}'
LIMIT_RESP=$(curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" -d "$LIMIT_PAYLOAD" \
    "${BASE}/api/banking/transfer" 2>/dev/null)
assert_output_contains "$LIMIT_RESP" "TRANSACTION_LIMIT_EXCEEDED" \
    "POST /api/banking/transfer over-limit returns TRANSACTION_LIMIT_EXCEEDED"

# Test: POST /api/banking/transfer — frozen account (ACC-1003 is frozen)
FROZEN_PAYLOAD='{"from_account":"ACC-1003","to_account":"ACC-1001","amount_cents":100}'
FROZEN_RESP=$(curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" -d "$FROZEN_PAYLOAD" \
    "${BASE}/api/banking/transfer" 2>/dev/null)
assert_output_contains "$FROZEN_RESP" "ACCOUNT_FROZEN" \
    "POST /api/banking/transfer frozen account returns ACCOUNT_FROZEN"

# Test: POST /api/banking/transfer — same account returns bad_request
SAME_PAYLOAD='{"from_account":"ACC-1001","to_account":"ACC-1001","amount_cents":100}'
SAME_RESP=$(curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" -d "$SAME_PAYLOAD" \
    "${BASE}/api/banking/transfer" 2>/dev/null)
assert_output_contains "$SAME_RESP" "same account" \
    "POST /api/banking/transfer same account returns error"

print_summary
