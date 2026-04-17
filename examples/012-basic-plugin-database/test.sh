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
API="$BASE/api/plugin-db"

echo "=== 012 Plugin Database ==="

if [ "${VIL_TEST_INFRA:-}" != "1" ]; then
    echo -e "  ${YELLOW}SKIP${NC} requires test infra (./infra/up.sh)"
    SKIP_COUNT=$((SKIP_COUNT + 1))
    return 0 2>/dev/null || exit 0
fi

# Init DB tables if missing
PG_URL="${DATABASE_URL:-${VIL_TEST_POSTGRES_URL:-postgres://viltest:viltest123@localhost:19432/vil_demo}}"
psql "$PG_URL" -q -c "
CREATE TABLE IF NOT EXISTS products (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT '',
    price DOUBLE PRECISION NOT NULL DEFAULT 0,
    stock INT NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS tasks (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    done BOOLEAN NOT NULL DEFAULT false
);
INSERT INTO products (name, category, price, stock)
SELECT 'Widget A', 'electronics', 29.99, 100
WHERE NOT EXISTS (SELECT 1 FROM products LIMIT 1);
" >/dev/null 2>&1

# Health
assert_http_status "$BASE/health" 200 "health endpoint"

# Plugins list
RESP=$(curl -s "$API/plugins")
assert_output_contains "$RESP" "postgres" "plugins has postgres"

# Config
assert_http_status "$API/config" 200 "config endpoint"

# Products (VilQuery)
RESP=$(curl -s "$API/products")
assert_output_contains "$RESP" "products" "products has products field"

# Create task
RESP=$(curl -s -X POST "$API/tasks" -H 'Content-Type: application/json' \
  -d '{"title":"test task","description":"from test suite"}')
assert_output_contains "$RESP" "id" "task created with id"

# List tasks
RESP=$(curl -s "$API/tasks")
assert_output_contains "$RESP" "test task" "tasks list contains our task"

# Pool stats
assert_http_status "$API/pool-stats" 200 "pool-stats endpoint"

# Redis ping
RESP=$(curl -s "$API/redis-ping")
assert_output_contains "$RESP" "pong" "redis-ping response"

# Cleanup test data
psql "$PG_URL" -q -c "DELETE FROM tasks WHERE title = 'test task';" >/dev/null 2>&1

print_summary
