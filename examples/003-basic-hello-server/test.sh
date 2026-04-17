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


echo "=== 003-basic-hello-server (Currency Exchange Service) ==="

PORT=${PORT:-8080}
BASE="http://127.0.0.1:${PORT}"

# Test: GET /api/fx/rates
assert_http_status "${BASE}/api/fx/rates" "200" \
    "GET /api/fx/rates returns HTTP 200"

RATES_RESP=$(curl -s --max-time 10 "${BASE}/api/fx/rates" 2>/dev/null)

assert_output_contains "$RATES_RESP" "USD" \
    "Rates response contains USD"
assert_output_contains "$RATES_RESP" "buy_rate" \
    "Rates response contains buy_rate"
assert_output_contains "$RATES_RESP" "sell_rate" \
    "Rates response contains sell_rate"
assert_output_contains "$RATES_RESP" "IDR" \
    "Rates response base is IDR"

# Test: POST /api/fx/convert (USD → IDR)
CONVERT_PAYLOAD='{"from":"USD","to":"IDR","amount":100.0}'
assert_http_status_post "${BASE}/api/fx/convert" "$CONVERT_PAYLOAD" "200" \
    "POST /api/fx/convert returns HTTP 200"

CONVERT_RESP=$(curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d "$CONVERT_PAYLOAD" \
    "${BASE}/api/fx/convert" 2>/dev/null)

assert_output_contains "$CONVERT_RESP" "converted_amount" \
    "Convert response contains converted_amount"
assert_output_contains "$CONVERT_RESP" "rate_applied" \
    "Convert response contains rate_applied"
assert_output_contains "$CONVERT_RESP" "conversion_id" \
    "Convert response contains conversion_id"

# Test: POST /api/fx/convert — invalid amount
BAD_PAYLOAD='{"from":"USD","to":"IDR","amount":-10}'
assert_http_status_post "${BASE}/api/fx/convert" "$BAD_PAYLOAD" "400" \
    "POST /api/fx/convert rejects negative amount (400)"

# Test: POST /api/fx/convert — unknown currency
UNK_PAYLOAD='{"from":"XYZ","to":"IDR","amount":100}'
assert_http_status_post "${BASE}/api/fx/convert" "$UNK_PAYLOAD" "404" \
    "POST /api/fx/convert rejects unknown currency (404)"

# Test: GET /api/fx/stats
assert_http_status "${BASE}/api/fx/stats" "200" \
    "GET /api/fx/stats returns HTTP 200"

STATS_RESP=$(curl -s --max-time 10 "${BASE}/api/fx/stats" 2>/dev/null)

assert_output_contains "$STATS_RESP" "total_conversions" \
    "Stats response contains total_conversions"
assert_output_contains "$STATS_RESP" "total_volume_idr" \
    "Stats response contains total_volume_idr"

print_summary
