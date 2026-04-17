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


echo "=== 608-db-vilorm-analytics (Event Analytics — VilORM) ==="

PORT=${PORT:-8088}
BASE="http://127.0.0.1:${PORT}"

# 1. Log events (VilQuery insert + on_conflict do_update_raw for counter)
for i in 1 2 3; do
    curl -s --max-time 10 -X POST \
        -H "Content-Type: application/json" \
        -d "{\"event_type\":\"page_view\",\"user_id\":\"user-${i}\",\"payload\":\"{\\\"page\\\":\\\"/home\\\"}\"}" \
        "${BASE}/api/analytics/events" > /dev/null 2>&1
done

curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d '{"event_type":"click","user_id":"user-1","payload":"{\"button\":\"signup\"}"}' \
    "${BASE}/api/analytics/events" > /dev/null 2>&1

EVENT_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d '{"event_type":"purchase","user_id":"user-2"}' \
    "${BASE}/api/analytics/events" 2>/dev/null)
assert_eq "$EVENT_STATUS" "201" "POST /events returns 201 (VilQuery insert)"

# 2. Recent events (VilQuery select + order_by_desc + limit)
RECENT_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/analytics/events/recent" 2>/dev/null)
RECENT_BODY=$(curl -s --max-time 10 \
    "${BASE}/api/analytics/events/recent" 2>/dev/null)
assert_eq "$RECENT_STATUS" "200" "GET /events/recent returns 200"
assert_output_contains "$RECENT_BODY" "page_view" "Recent events contains page_view"

# 3. Events by type (VilQuery GROUP BY)
TYPE_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/analytics/events/by-type" 2>/dev/null)
TYPE_BODY=$(curl -s --max-time 10 \
    "${BASE}/api/analytics/events/by-type" 2>/dev/null)
assert_eq "$TYPE_STATUS" "200" "GET /events/by-type returns 200 (GROUP BY)"
assert_output_contains "$TYPE_BODY" "event_type" "By-type contains event_type field"

# 4. Daily stats (VilQuery GROUP BY date + ORDER BY — time series)
DAILY_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/analytics/stats/daily" 2>/dev/null)
assert_eq "$DAILY_STATUS" "200" "GET /stats/daily returns 200 (time series GROUP BY)"

# 5. Unique users (VilQuery COUNT DISTINCT scalar)
UNIQ_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/analytics/stats/unique-users" 2>/dev/null)
UNIQ_BODY=$(curl -s --max-time 10 \
    "${BASE}/api/analytics/stats/unique-users" 2>/dev/null)
assert_eq "$UNIQ_STATUS" "200" "GET /stats/unique-users returns 200 (COUNT DISTINCT)"
assert_output_contains "$UNIQ_BODY" "unique_users" "Unique users contains count"

# 6. Summary (multiple VilQuery scalar aggregates)
SUM_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/analytics/stats/summary" 2>/dev/null)
SUM_BODY=$(curl -s --max-time 10 \
    "${BASE}/api/analytics/stats/summary" 2>/dev/null)
assert_eq "$SUM_STATUS" "200" "GET /stats/summary returns 200"
assert_output_contains "$SUM_BODY" "total_events" "Summary contains total_events"
assert_output_contains "$SUM_BODY" "unique_users" "Summary contains unique_users"

print_summary
