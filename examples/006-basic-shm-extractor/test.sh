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


echo "=== 006-basic-shm-extractor (VilApp) ==="

PORT=${PORT:-8080}
BASE="http://127.0.0.1:${PORT}"
INGEST_PAYLOAD="$SCRIPT_DIR/payloads/ingest.json"

# Test: POST /api/shm-demo/ingest
assert_http_status_post "${BASE}/api/shm-demo/ingest" "$(cat "$INGEST_PAYLOAD")" "200" \
    "POST /api/shm-demo/ingest returns HTTP 200"

INGEST_RESP=$(curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d @"$INGEST_PAYLOAD" \
    "${BASE}/api/shm-demo/ingest" 2>/dev/null)

assert_output_contains "$INGEST_RESP" "sensor_id\|T1\|ok\|success" \
    "POST /api/shm-demo/ingest returns valid response"

# Test: POST /api/shm-demo/compute (requires JSON body with iterations)
COMPUTE_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    -X POST -H "Content-Type: application/json" \
    -d '{"iterations":100000}' \
    "${BASE}/api/shm-demo/compute" 2>/dev/null)

assert_eq "$COMPUTE_STATUS" "200" "POST /api/shm-demo/compute returns HTTP 200"

# Test: GET /api/shm-demo/shm-stats
assert_http_status "${BASE}/api/shm-demo/shm-stats" "200" \
    "GET /api/shm-demo/shm-stats returns HTTP 200"

STATS_RESP=$(curl -s --max-time 10 \
    "${BASE}/api/shm-demo/shm-stats" 2>/dev/null)

assert_output_contains "$STATS_RESP" "shm_available" \
    "GET /api/shm-demo/shm-stats response contains 'shm_available'"

# Test: GET /api/shm-demo/benchmark
assert_http_status "${BASE}/api/shm-demo/benchmark" "200" \
    "GET /api/shm-demo/benchmark returns HTTP 200"

print_summary
