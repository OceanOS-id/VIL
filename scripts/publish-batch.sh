#!/usr/bin/env bash
# =============================================================================
# VIL Batch Publish — 10 crates per run, 2 min between each
# =============================================================================
# Usage:
#   # Run once (publishes up to 10 new crates):
#   ./scripts/publish-batch.sh
#
#   # Loop until all done:
#   while ./scripts/publish-batch.sh; do echo "--- Waiting 10 min ---"; sleep 600; done
# =============================================================================

set -uo pipefail
cd "$(dirname "$0")/.."

# Load shared CRATES + SKIP + version check helpers
# shellcheck source=./publish-common.sh
source scripts/publish-common.sh

# Version preflight
publish_preflight_version || exit 1

BATCH=10
SKIP="$PUBLISH_SKIP_CRATES"
PUBLISHED=0
RATE_LIMITED=0
FAILED=0
REMAINING=0

# ─────────────────────────────────────────────────────────────────────────────
# Dependency order — verified 2026-03-28
# Rule: every crate's VIL deps must appear ABOVE it in this list.
#
# KEY FIXES (observer integration):
#   - vil_observer bumped to 0.1.1 (added vil_log + connector_event deps)
#   - vil_server_core bumped to 0.1.1 (observer wiring)
#   - vil_new_http + vil_sdk BEFORE vil_server (was wrong before)
#   - vil_plugin_sdk BEFORE vil_server (only needs server_core)
# ─────────────────────────────────────────────────────────────────────────────
# CRATES list delegated to scripts/publish-common.sh (single source)
CRATES=("${PUBLISH_CRATES[@]}")

# ─────────────────────────────────────────────────────────────────────────────
# Helper: read version from crate's Cargo.toml
# ─────────────────────────────────────────────────────────────────────────────
get_local_version() {
    local crate="$1"
    local toml="crates/$crate/Cargo.toml"
    if [[ ! -f "$toml" ]]; then
        echo "0.0.0"
        return
    fi
    # Handle both `version = "x.y.z"` and `version.workspace = true`
    local ver
    ver=$(grep '^version' "$toml" | head -1)
    if echo "$ver" | grep -q 'workspace'; then
        # Read workspace version from root Cargo.toml
        ver=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
    else
        ver=$(echo "$ver" | sed 's/.*"\(.*\)"/\1/')
    fi
    echo "$ver"
}

# Deduplicate
declare -A SEEN
UNIQUE=()
for c in "${CRATES[@]}"; do
    if [[ -z "${SEEN[$c]:-}" ]]; then
        SEEN[$c]=1
        UNIQUE+=("$c")
    fi
done

echo "=== VIL Batch Publish — $(date) ==="

for crate in "${UNIQUE[@]}"; do
    # Skip list
    for skip in $SKIP; do
        [[ "$crate" == "$skip" ]] && continue 2
    done

    # Read local version and check crates.io
    local_ver=$(get_local_version "$crate")
    status=$(curl -s -o /dev/null -w "%{http_code}" "https://crates.io/api/v1/crates/$crate/$local_ver")
    if [[ "$status" == "200" ]]; then
        continue  # already published at this version
    fi

    REMAINING=$((REMAINING + 1))

    # Publish (with 1 retry on transient failures)
    echo -n "  Publishing $crate v$local_ver ... "
    attempt=0
    published=false
    while [[ $attempt -lt 2 ]]; do
        output=$(cargo publish -p "$crate" --allow-dirty 2>&1)
        if echo "$output" | grep -q "Published\|Uploaded"; then
            echo "✅"
            PUBLISHED=$((PUBLISHED + 1))
            published=true
            break
        elif echo "$output" | grep -q "already exists"; then
            echo "⏭ (exists)"
            published=true
            break
        elif echo "$output" | grep -q "429\|Too Many Requests"; then
            echo "⏳ rate limited — stopping batch"
            RATE_LIMITED=1
            break 2  # exit outer for loop
        elif echo "$output" | grep -q "503\|x-timer\|timed out\|Service Unavailable"; then
            # Transient crates.io error — retry once after 30s
            attempt=$((attempt + 1))
            if [[ $attempt -lt 2 ]]; then
                echo -n "⚠ transient error, retry in 30s ... "
                sleep 30
            fi
        else
            break  # real error, don't retry
        fi
    done

    if [[ "$published" != "true" && $RATE_LIMITED -eq 0 ]]; then
        echo "❌"
        echo "$output" | grep -v "^$" | tail -4 | sed 's/^/    /'
        FAILED=$((FAILED + 1))
    fi

    if [[ $PUBLISHED -ge $BATCH ]]; then
        echo "  Batch of $BATCH done."
        break
    fi

    # 10.000-15.999s random delay (millisecond precision)
    delay_s=$((RANDOM % 6 + 10))
    delay_ms=$((RANDOM % 1000))
    sleep "${delay_s}.$(printf '%03d' $delay_ms)"
done

echo ""
echo "Published this batch: $PUBLISHED | Failed: $FAILED | Rate limited: $RATE_LIMITED"

if [[ $REMAINING -eq 0 && $RATE_LIMITED -eq 0 ]]; then
    echo "=== All crates published! ==="
    exit 1
elif [[ $PUBLISHED -gt 0 || $RATE_LIMITED -gt 0 ]]; then
    echo "=== More to do — will retry after cooldown ==="
    exit 0
else
    echo "=== Some crates failed (dependency order?) — will retry ==="
    exit 0
fi
