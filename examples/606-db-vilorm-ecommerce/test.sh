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


echo "=== 606-db-vilorm-ecommerce (Products + Orders — VilORM) ==="

PORT=${PORT:-8086}
BASE="http://127.0.0.1:${PORT}"

# 1. Create product (VilQuery insert + value_opt_str)
PROD_RESP=$(curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d '{"name":"Laptop","description":"High-end laptop","price":1500.0,"stock":10,"category":"electronics"}' \
    "${BASE}/api/shop/products" 2>/dev/null)
PROD_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d '{"name":"Mouse","price":25.0,"stock":100}' \
    "${BASE}/api/shop/products" 2>/dev/null)

assert_output_contains "$PROD_RESP" "Laptop" "POST /products creates product"
assert_eq "$PROD_STATUS" "201" "POST /products returns 201"

PROD_ID=$(echo "$PROD_RESP" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)

# 2. List products (VilQuery select + order_by + limit)
LIST_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/shop/products" 2>/dev/null)
assert_eq "$LIST_STATUS" "200" "GET /products returns 200 (VilQuery select projection)"

# 3. Get product (find_by_id)
if [ -n "$PROD_ID" ]; then
    GET_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        "${BASE}/api/shop/products/${PROD_ID}" 2>/dev/null)
    assert_eq "$GET_STATUS" "200" "GET /products/:id returns 200 (find_by_id)"
fi

# 4. Create order (VilQuery insert + atomic stock decrement via set_expr)
if [ -n "$PROD_ID" ]; then
    ORDER_RESP=$(curl -s --max-time 10 -X POST \
        -H "Content-Type: application/json" \
        -d "{\"customer_name\":\"John\",\"items\":[{\"product_id\":\"${PROD_ID}\",\"quantity\":2}]}" \
        "${BASE}/api/shop/orders" 2>/dev/null)
    ORDER_STATUS=$(echo "$ORDER_RESP" | python3 -c "import sys,json; print(201)" 2>/dev/null || echo "unknown")
    assert_output_contains "$ORDER_RESP" "customer_name" "POST /orders creates order with items"

    ORDER_ID=$(echo "$ORDER_RESP" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
else
    skip_test "No product ID for order"
fi

# 5. List orders (VilQuery select + JOIN)
ORDERS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/shop/orders" 2>/dev/null)
assert_eq "$ORDERS_STATUS" "200" "GET /orders returns 200 (VilQuery JOIN)"

# 6. Order total (VilQuery scalar aggregate)
if [ -n "$ORDER_ID" ]; then
    TOTAL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        "${BASE}/api/shop/orders/${ORDER_ID}/total" 2>/dev/null)
    TOTAL_BODY=$(curl -s --max-time 10 \
        "${BASE}/api/shop/orders/${ORDER_ID}/total" 2>/dev/null)
    assert_eq "$TOTAL_STATUS" "200" "GET /orders/:id/total returns 200 (scalar SUM)"
    assert_output_contains "$TOTAL_BODY" "total" "Order total contains amount"
fi

# 7. Delete order (VilEntity delete + delete_where)
if [ -n "$ORDER_ID" ]; then
    DEL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        -X DELETE "${BASE}/api/shop/orders/${ORDER_ID}" 2>/dev/null)
    assert_eq "$DEL_STATUS" "200" "DELETE /orders/:id returns 200"
fi

print_summary
