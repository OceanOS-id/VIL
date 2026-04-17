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


echo "=== 004-basic-rest-crud (VilORM + SQLite) ==="

PORT=${PORT:-8080}
BASE="http://127.0.0.1:${PORT}"

# Test: POST /api/tasks/tasks — create a task via VilQuery insert
CREATE_RESP=$(curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d '{"title":"Test Task","description":"Created by testsuite"}' \
    -w "\n%{http_code}" \
    "${BASE}/api/tasks/tasks" 2>/dev/null)

CREATE_STATUS=$(echo "$CREATE_RESP" | tail -1)
CREATE_BODY=$(echo "$CREATE_RESP" | sed '$d')

assert_eq "$CREATE_STATUS" "201" "POST /tasks returns HTTP 201 (VilQuery insert)"
assert_output_contains "$CREATE_BODY" "Test Task" \
    "POST /tasks response contains title"

# Extract UUID task ID
TASK_ID=$(echo "$CREATE_BODY" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)

# Test: GET /api/tasks/tasks — list via VilQuery select (slim projection)
LIST_RESP=$(curl -s --max-time 10 "${BASE}/api/tasks/tasks" 2>/dev/null)
LIST_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/tasks/tasks" 2>/dev/null)

assert_eq "$LIST_STATUS" "200" "GET /tasks returns HTTP 200 (VilQuery select)"
assert_output_contains "$LIST_RESP" "Test Task" "GET /tasks list contains created task"

# Test: GET /api/tasks/tasks/:id — find_by_id
if [ -n "$TASK_ID" ]; then
    GET_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        "${BASE}/api/tasks/tasks/${TASK_ID}" 2>/dev/null)
    assert_eq "$GET_STATUS" "200" "GET /tasks/:id returns HTTP 200 (find_by_id)"

    # Test: PUT — VilQuery update().set_optional()
    PUT_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        -X PUT -H "Content-Type: application/json" \
        -d '{"done":true}' \
        "${BASE}/api/tasks/tasks/${TASK_ID}" 2>/dev/null)
    assert_eq "$PUT_STATUS" "200" "PUT /tasks/:id returns HTTP 200 (set_optional update)"

    # Test: GET /api/tasks/tasks/stats — VilQuery scalar aggregate
    STATS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        "${BASE}/api/tasks/tasks/stats" 2>/dev/null)
    assert_eq "$STATS_STATUS" "200" "GET /tasks/stats returns HTTP 200 (scalar aggregate)"

    # Test: DELETE — T::delete()
    DEL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        -X DELETE "${BASE}/api/tasks/tasks/${TASK_ID}" 2>/dev/null)
    assert_eq "$DEL_STATUS" "200" "DELETE /tasks/:id returns HTTP 200 (VilEntity delete)"
else
    skip_test "Could not extract task ID from create response"
fi

print_summary
