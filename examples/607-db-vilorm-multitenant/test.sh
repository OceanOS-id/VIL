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


echo "=== 607-db-vilorm-multitenant (SaaS Tenants — VilORM) ==="

PORT=${PORT:-8087}
BASE="http://127.0.0.1:${PORT}"

# 1. Create tenant (VilQuery insert)
TENANT_RESP=$(curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d '{"name":"Acme Corp","plan":"pro"}' \
    "${BASE}/api/saas/tenants" 2>/dev/null)
assert_output_contains "$TENANT_RESP" "Acme" "POST /tenants creates tenant"

TENANT_ID=$(echo "$TENANT_RESP" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$TENANT_ID" ]; then
    skip_test "No tenant ID"
    print_summary
    exit $?
fi

# 2. Get tenant (find_by_id)
GET_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/saas/tenants/${TENANT_ID}" 2>/dev/null)
assert_eq "$GET_STATUS" "200" "GET /tenants/:id returns 200 (find_by_id)"

# 3. Update tenant (VilQuery update().set_optional() — partial update)
PUT_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    -X PUT -H "Content-Type: application/json" \
    -d '{"name":"Acme Inc"}' \
    "${BASE}/api/saas/tenants/${TENANT_ID}" 2>/dev/null)
assert_eq "$PUT_STATUS" "200" "PUT /tenants/:id returns 200 (set_optional partial update)"

# 4. Add user (VilQuery insert + on_conflict_nothing — idempotent)
USER_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    -X POST -H "Content-Type: application/json" \
    -d '{"email":"alice@acme.com","role":"admin"}' \
    "${BASE}/api/saas/tenants/${TENANT_ID}/users" 2>/dev/null)
assert_eq "$USER_STATUS" "200" "POST /tenants/:id/users returns 200 (insert)"

# Idempotent — same email should not fail
USER_STATUS2=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    -X POST -H "Content-Type: application/json" \
    -d '{"email":"alice@acme.com","role":"admin"}' \
    "${BASE}/api/saas/tenants/${TENANT_ID}/users" 2>/dev/null)
assert_eq "$USER_STATUS2" "200" "POST /tenants/:id/users idempotent (on_conflict_nothing)"

# 5. List users scoped to tenant (VilQuery where_eq)
USERS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/saas/tenants/${TENANT_ID}/users" 2>/dev/null)
USERS_BODY=$(curl -s --max-time 10 \
    "${BASE}/api/saas/tenants/${TENANT_ID}/users" 2>/dev/null)
assert_eq "$USERS_STATUS" "200" "GET /tenants/:id/users returns 200 (scoped query)"
assert_output_contains "$USERS_BODY" "alice@acme.com" "Users list contains added user"

# 6. Upsert setting (VilQuery on_conflict().do_update())
SETTING_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    -X POST -H "Content-Type: application/json" \
    -d '{"key":"theme","value":"dark"}' \
    "${BASE}/api/saas/tenants/${TENANT_ID}/settings" 2>/dev/null)
assert_eq "$SETTING_STATUS" "200" "POST /settings returns 200 (upsert)"

# Update same key (ON CONFLICT DO UPDATE)
SETTING_STATUS2=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    -X POST -H "Content-Type: application/json" \
    -d '{"key":"theme","value":"light"}' \
    "${BASE}/api/saas/tenants/${TENANT_ID}/settings" 2>/dev/null)
assert_eq "$SETTING_STATUS2" "200" "POST /settings upsert updates existing (on_conflict do_update)"

# 7. List settings
SETTINGS_BODY=$(curl -s --max-time 10 \
    "${BASE}/api/saas/tenants/${TENANT_ID}/settings" 2>/dev/null)
assert_output_contains "$SETTINGS_BODY" "light" "Settings contains updated value"

# 8. Stats (VilQuery scalar — COUNT per tenant)
STATS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/saas/tenants/${TENANT_ID}/stats" 2>/dev/null)
STATS_BODY=$(curl -s --max-time 10 \
    "${BASE}/api/saas/tenants/${TENANT_ID}/stats" 2>/dev/null)
assert_eq "$STATS_STATUS" "200" "GET /stats returns 200 (scalar COUNT)"
assert_output_contains "$STATS_BODY" "user_count" "Stats contains user_count"

print_summary
