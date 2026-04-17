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

echo "=== 308 RAG Full Pipeline Ingest+Query ==="

# ── Infra check ──
if ! curl -s -o /dev/null -w "%{http_code}" http://localhost:4545/v1/chat/completions -X POST \
    -H "Content-Type: application/json" -d '{"model":"gpt-4","messages":[{"role":"user","content":"ping"}]}' 2>/dev/null | grep -q "200"; then
    echo -e "  ${YELLOW}SKIP${NC} ai-endpoint-simulator not running on :4545"
    SKIP_COUNT=$((SKIP_COUNT + 1))
    return 0 2>/dev/null || exit 0
fi

# Health
assert_http_status "$BASE/health" 200 "health endpoint"

# Ingest document
RESP=$(curl -s -X POST "$BASE/api/rag/ingest" -H 'Content-Type: application/json' \
  -d '{"doc_id":"DOC-TEST-001","title":"VIL Architecture","content":"VIL is a zero-copy streaming pipeline framework for Rust."}')
assert_output_contains "$RESP" "doc_id" "ingest has doc_id"
assert_output_contains "$RESP" "chunks_stored" "ingest has chunks_stored"

# Query
RESP=$(curl -s -X POST "$BASE/api/rag/query" -H 'Content-Type: application/json' \
  -d '{"question":"What is VIL?","top_k":3}' --max-time 15)
if [ -n "$RESP" ]; then
    assert_output_contains "$RESP" "answer" "query has answer"
    assert_output_contains "$RESP" "sources" "query has sources"
else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    FAILURES+=("308: POST /api/rag/query empty")
    echo -e "  ${RED}FAIL${NC} POST /api/rag/query empty"
fi

# Stats
assert_http_status "$BASE/api/rag/stats" 200 "stats endpoint"

print_summary
