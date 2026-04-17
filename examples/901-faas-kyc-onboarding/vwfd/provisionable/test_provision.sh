#!/bin/bash
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0; FAILURES=()
GREEN="\033[0;32m"; RED="\033[0;31m"; NC="\033[0m"

BASE="${PROVISION_URL:-http://localhost:19080}"
echo "=== 901-faas-kyc-onboarding (provision) ==="

RESP=$(curl -s --max-time 10 -X POST "$BASE/api/kyc/verify" \
    -H 'Content-Type: application/json' \
    -d '{"name":"John Doe","phone":"+6281234567890","email":"john@example.com"}')
[ -n "$RESP" ] && { PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} POST /api/kyc/verify"; } \
    || { FAIL_COUNT=$((FAIL_COUNT+1)); FAILURES+=("POST /api/kyc/verify empty"); echo -e "  ${RED}FAIL${NC} empty"; }
