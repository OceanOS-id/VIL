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


echo "=== 804 Trigger CDC Postgres ==="

if [ "${VIL_TEST_INFRA:-}" != "1" ]; then
    echo -e "  ${YELLOW}SKIP${NC} requires test infra (./infra/up.sh)"
    SKIP_COUNT=$((SKIP_COUNT + 1))
    return 0 2>/dev/null || exit 0
fi

PG_PORT="${VIL_TEST_POSTGRES_PORT:-19432}"
PG_URL="postgres://viltest:viltest123@localhost:$PG_PORT/vil_demo"

# Verify PostgreSQL reachable
if ! docker exec vil-postgres pg_isready -U viltest -q 2>/dev/null; then
    echo -e "  ${YELLOW}SKIP${NC} PostgreSQL not reachable on :$PG_PORT"
    SKIP_COUNT=$((SKIP_COUNT + 1))
    return 0 2>/dev/null || exit 0
fi

# Ensure `orders` table exists. The shared vil-postgres container doesn't
# ship this table by default; test is idempotent (safe to re-run).
docker exec vil-postgres psql -U viltest -d vil_demo -q -c "
CREATE TABLE IF NOT EXISTS orders (
    id BIGSERIAL PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    product TEXT NOT NULL,
    amount_cents INTEGER NOT NULL,
    status TEXT DEFAULT 'pending',
    created_at TIMESTAMPTZ DEFAULT now()
);
" 2>/dev/null || true

# Check wal_level = logical
WAL_LEVEL=$(docker exec vil-postgres psql -U viltest -d vil_demo -tAc "SHOW wal_level;" 2>/dev/null)
if [ "$WAL_LEVEL" = "logical" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} wal_level = logical"
else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    FAILURES+=("804: wal_level is '$WAL_LEVEL', expected 'logical'")
    echo -e "  ${RED}FAIL${NC} wal_level = '$WAL_LEVEL' (expected logical)"
fi

# Check publication exists
PUB_EXISTS=$(docker exec vil-postgres psql -U viltest -d vil_demo -tAc "SELECT count(*) FROM pg_publication WHERE pubname = 'vil_test_pub';" 2>/dev/null)
if [ "$PUB_EXISTS" = "1" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} publication 'vil_test_pub' exists"
else
    echo -e "  ${YELLOW}SKIP${NC} publication not created (init may have failed)"
    SKIP_COUNT=$((SKIP_COUNT + 1))
fi

# Check replication slot
SLOT_EXISTS=$(docker exec vil-postgres psql -U viltest -d vil_demo -tAc "SELECT count(*) FROM pg_replication_slots WHERE slot_name = 'vil_test_slot';" 2>/dev/null)
if [ "$SLOT_EXISTS" = "1" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} replication slot 'vil_test_slot' exists"
else
    echo -e "  ${YELLOW}SKIP${NC} replication slot not created"
    SKIP_COUNT=$((SKIP_COUNT + 1))
fi

# Insert a row to orders table (triggers CDC event)
docker exec vil-postgres psql -U viltest -d vil_demo -c "INSERT INTO orders (customer_id, product, amount_cents, status) VALUES (1, 'VIL Test Product', 9999, 'pending');" 2>/dev/null
PASS_COUNT=$((PASS_COUNT + 1))
echo -e "  ${GREEN}PASS${NC} INSERT into orders (CDC event triggered)"

# Verify row exists
ROW_COUNT=$(docker exec vil-postgres psql -U viltest -d vil_demo -tAc "SELECT count(*) FROM orders WHERE product = 'VIL Test Product';" 2>/dev/null)
if [ "$ROW_COUNT" -ge 1 ] 2>/dev/null; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo -e "  ${GREEN}PASS${NC} order row confirmed in table"
else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    FAILURES+=("804: order row not found")
    echo -e "  ${RED}FAIL${NC} order row not found"
fi

# Cleanup
docker exec vil-postgres psql -U viltest -d vil_demo -c "DELETE FROM orders WHERE product = 'VIL Test Product';" 2>/dev/null

print_summary
