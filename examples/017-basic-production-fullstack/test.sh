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


BASE="http://localhost:8080"

echo "=== 017 Production Fullstack ==="

# Health
assert_http_status "$BASE/health" 200 "health endpoint"

# Platform config
RESP=$(curl -s "$BASE/api/platform/config")
assert_output_contains "$RESP" "server_name" "platform config has server_name"
assert_output_contains "$RESP" "observer_enabled" "platform config has observer_enabled"

# Sprint list (seeded with sample data)
RESP=$(curl -s "$BASE/api/sprints/list")
assert_output_contains "$RESP" "title" "sprint list has title"
assert_output_contains "$RESP" "status" "sprint list has status"
assert_output_contains "$RESP" "assignee" "sprint list has assignee"

# Sprint stats
RESP=$(curl -s "$BASE/api/sprints/stats")
assert_output_contains "$RESP" "total" "sprint stats has total"
assert_output_contains "$RESP" "velocity_pct" "sprint stats has velocity_pct"
assert_output_contains "$RESP" "completed_story_points" "sprint stats has completed_story_points"

# Create sprint
RESP=$(curl -s -X POST "$BASE/api/sprints/create" -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer vil-demo-token-2026' \
  -d '{"title":"Test sprint from testsuite","status":"planned","assignee":"tester","story_points":5}')
assert_output_contains "$RESP" "title" "create sprint has title"
assert_output_contains "$RESP" "id" "create sprint has id"

# Update sprint (update sprint id 1)
RESP=$(curl -s -X PUT "$BASE/api/sprints/update" -H 'Content-Type: application/json' \
  -d '{"id":1,"status":"done"}')
assert_output_contains "$RESP" "status" "update sprint has status"

print_summary
