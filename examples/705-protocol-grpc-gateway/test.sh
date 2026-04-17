#!/bin/bash
# Self-contained test — no external dependencies
# Requires: grpcurl (https://github.com/fullstorydev/grpcurl)
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
assert_output_contains() {
    local output="$1" pattern="$2" msg="${3:-contains $2}"
    if echo "$output" | grep -q "$pattern"; then
        PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} $msg"
    else
        FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("$msg"); echo -e "  ${RED}FAIL${NC} $msg"
    fi
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

echo "=== 705-protocol-grpc-gateway ==="

GRPC_ADDR="${GRPC_ADDR:-127.0.0.1:50051}"

if ! command -v grpcurl &>/dev/null; then
    skip_test "grpcurl not installed — cannot test gRPC endpoints"
    print_summary
    exit 0
fi

# Test 1: Charge RPC
CHARGE_RESP=$(grpcurl -plaintext -proto "$SCRIPT_DIR/proto/payment.proto" -import-path "$SCRIPT_DIR/proto" \
    -d '{"customer_id":"C-001","amount_cents":5000,"currency":"USD","description":"Order #1234"}' \
    "$GRPC_ADDR" payment.PaymentService/Charge 2>/dev/null)
CHARGE_EXIT=$?

assert_eq "$CHARGE_EXIT" "0" "Charge RPC succeeds (exit 0)"
assert_output_contains "$CHARGE_RESP" "paymentId" "Charge response contains paymentId"
assert_output_contains "$CHARGE_RESP" "approved" "Charge status is approved"
assert_output_contains "$CHARGE_RESP" "C-001" "Charge echoes customer_id"

# Extract payment_id for subsequent tests (grpcurl outputs camelCase JSON)
PAYMENT_ID=$(echo "$CHARGE_RESP" | grep -o '"paymentId": *"[^"]*"' | head -1 | sed 's/.*"paymentId": *"\([^"]*\)".*/\1/')

if [ -z "$PAYMENT_ID" ]; then
    skip_test "Could not extract paymentId — skipping GetPayment and Refund"
    print_summary
    exit 0
fi

# Test 2: GetPayment RPC
GET_RESP=$(grpcurl -plaintext -proto "$SCRIPT_DIR/proto/payment.proto" -import-path "$SCRIPT_DIR/proto" \
    -d "{\"payment_id\":\"${PAYMENT_ID}\"}" \
    "$GRPC_ADDR" payment.PaymentService/GetPayment 2>/dev/null)
GET_EXIT=$?

assert_eq "$GET_EXIT" "0" "GetPayment RPC succeeds (exit 0)"
assert_output_contains "$GET_RESP" "$PAYMENT_ID" "GetPayment returns correct paymentId"
assert_output_contains "$GET_RESP" "approved" "GetPayment status is approved"

# Test 3: Refund RPC
REFUND_RESP=$(grpcurl -plaintext -proto "$SCRIPT_DIR/proto/payment.proto" -import-path "$SCRIPT_DIR/proto" \
    -d "{\"payment_id\":\"${PAYMENT_ID}\",\"reason\":\"customer request\"}" \
    "$GRPC_ADDR" payment.PaymentService/Refund 2>/dev/null)
REFUND_EXIT=$?

assert_eq "$REFUND_EXIT" "0" "Refund RPC succeeds (exit 0)"
assert_output_contains "$REFUND_RESP" "refunded" "Refund status is refunded"
assert_output_contains "$REFUND_RESP" "$PAYMENT_ID" "Refund echoes paymentId"

# Test 4: Refund idempotency — second refund should fail
REFUND2_RESP=$(grpcurl -plaintext -proto "$SCRIPT_DIR/proto/payment.proto" -import-path "$SCRIPT_DIR/proto" \
    -d "{\"payment_id\":\"${PAYMENT_ID}\",\"reason\":\"duplicate\"}" \
    "$GRPC_ADDR" payment.PaymentService/Refund 2>&1)

assert_output_contains "$REFUND2_RESP" "already refunded" "Double refund rejected"

# Test 5: Charge > $10,000 declined
DECLINE_RESP=$(grpcurl -plaintext -proto "$SCRIPT_DIR/proto/payment.proto" -import-path "$SCRIPT_DIR/proto" \
    -d '{"customer_id":"C-002","amount_cents":1500000,"currency":"USD","description":"Big order"}' \
    "$GRPC_ADDR" payment.PaymentService/Charge 2>/dev/null)

assert_output_contains "$DECLINE_RESP" "declined" "Charge > $10,000 is declined"

print_summary
