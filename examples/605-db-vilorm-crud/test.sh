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


echo "=== 605-db-vilorm-crud (Blog Platform — VilORM showcase) ==="

PORT=${PORT:-8080}
BASE="http://127.0.0.1:${PORT}"

# 1. Create author (VilQuery insert + value_opt_str for nullable bio)
AUTHOR_RESP=$(curl -s --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d '{"name":"Alice","bio":"Rust enthusiast"}' \
    "${BASE}/api/blog/authors" 2>/dev/null)
AUTHOR_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 -X POST \
    -H "Content-Type: application/json" \
    -d '{"name":"Bob"}' \
    "${BASE}/api/blog/authors" 2>/dev/null)

assert_output_contains "$AUTHOR_RESP" "Alice" "POST /authors creates author (VilQuery insert + value_opt_str)"
assert_eq "$AUTHOR_STATUS" "201" "POST /authors returns 201"

AUTHOR_ID=$(echo "$AUTHOR_RESP" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)

# 2. List authors (VilEntity find_all)
AUTHORS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/blog/authors" 2>/dev/null)
assert_eq "$AUTHORS_STATUS" "200" "GET /authors returns 200 (find_all)"

# 3. Create post (VilQuery insert + set_raw for counter increment)
if [ -n "$AUTHOR_ID" ]; then
    POST_RESP=$(curl -s --max-time 10 -X POST \
        -H "Content-Type: application/json" \
        -d "{\"author_id\":\"${AUTHOR_ID}\",\"title\":\"VilORM Guide\",\"content\":\"Full tutorial\",\"status\":\"published\"}" \
        "${BASE}/api/blog/posts" 2>/dev/null)
    assert_output_contains "$POST_RESP" "VilORM Guide" "POST /posts creates post"
    POST_ID=$(echo "$POST_RESP" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
else
    skip_test "No author ID"
fi

# 4. List posts (VilQuery JOIN — select with alias + join)
POSTS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/blog/posts" 2>/dev/null)
POSTS_BODY=$(curl -s --max-time 10 "${BASE}/api/blog/posts" 2>/dev/null)
assert_eq "$POSTS_STATUS" "200" "GET /posts returns 200 (VilQuery JOIN)"
assert_output_contains "$POSTS_BODY" "author_name" "GET /posts contains author_name from JOIN"

# 5. Get single post (find_by_id + views increment via set_raw)
if [ -n "$POST_ID" ]; then
    GET_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        "${BASE}/api/blog/posts/${POST_ID}" 2>/dev/null)
    assert_eq "$GET_STATUS" "200" "GET /posts/:id returns 200 (find_by_id + views++)"

    # 6. Update post (VilQuery update().set_optional())
    PUT_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        -X PUT -H "Content-Type: application/json" \
        -d '{"title":"VilORM Guide v2"}' \
        "${BASE}/api/blog/posts/${POST_ID}" 2>/dev/null)
    assert_eq "$PUT_STATUS" "200" "PUT /posts/:id returns 200 (set_optional partial update)"
fi

# 7. Create tag (VilQuery on_conflict_nothing — idempotent upsert)
TAG_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    -X POST -H "Content-Type: application/json" \
    -d '{"name":"rust"}' \
    "${BASE}/api/blog/tags" 2>/dev/null)
assert_eq "$TAG_STATUS" "200" "POST /tags returns 200 (on_conflict_nothing)"

# Idempotent — second call should not fail
TAG_STATUS2=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    -X POST -H "Content-Type: application/json" \
    -d '{"name":"rust"}' \
    "${BASE}/api/blog/tags" 2>/dev/null)
assert_eq "$TAG_STATUS2" "200" "POST /tags idempotent (on_conflict_nothing)"

# 8. Stats (VilQuery scalar aggregates)
STATS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
    "${BASE}/api/blog/stats" 2>/dev/null)
STATS_BODY=$(curl -s --max-time 10 "${BASE}/api/blog/stats" 2>/dev/null)
assert_eq "$STATS_STATUS" "200" "GET /stats returns 200 (scalar aggregates)"
assert_output_contains "$STATS_BODY" "total_posts" "Stats contains total_posts"

# 9. Delete post (VilEntity delete)
if [ -n "$POST_ID" ]; then
    DEL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 \
        -X DELETE "${BASE}/api/blog/posts/${POST_ID}" 2>/dev/null)
    assert_eq "$DEL_STATUS" "200" "DELETE /posts/:id returns 200 (VilEntity delete)"
fi

print_summary
