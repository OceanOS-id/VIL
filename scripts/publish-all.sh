#!/usr/bin/env bash
# =============================================================================
# VIL Crates Publish Script (v2 — proper error handling)
# =============================================================================
# Publishes all VIL crates to crates.io in correct dependency order.
#
# Features:
#   - Clear error reporting: shows FULL error output, categorized
#   - Log file: all output saved to publish-log-{timestamp}.txt
#   - Smart retry: only retry rate limits, not compile/version errors
#   - Progress tracking: [N/TOTAL] with elapsed time
#   - Resume: skips already-published crates automatically
#
# Usage:
#   ./scripts/publish-all.sh             # Real publish
#   ./scripts/publish-all.sh --dry-run   # Dry run
# =============================================================================

set -uo pipefail

cd "$(dirname "$0")/.."

# Load shared CRATES + SKIP + version check helpers
# shellcheck source=./publish-common.sh
source scripts/publish-common.sh

# ---------------------------------------------------------------------------
# Arguments
# ---------------------------------------------------------------------------
DRY_RUN=""
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN="--dry-run"
    echo "=== DRY RUN MODE ==="
fi

# ---------------------------------------------------------------------------
# Version preflight — assert workspace version matches expected release tag
# ---------------------------------------------------------------------------
publish_preflight_version || exit 1

# ---------------------------------------------------------------------------
# Log file
# ---------------------------------------------------------------------------
LOG_DIR="$(pwd)/logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/publish-$(date +%Y%m%d-%H%M%S).log"
echo "Log: $LOG_FILE"

# Tee all output to log file AND terminal
exec > >(tee -a "$LOG_FILE") 2>&1

# ---------------------------------------------------------------------------
# Config (inherited from publish-common.sh; aliased for backward compatibility)
# ---------------------------------------------------------------------------
SKIP_CRATES="$PUBLISH_SKIP_CRATES"
WAIT_SECONDS=45
BATCH_SIZE=5
BATCH_COOLDOWN=660  # 11 min

# ---------------------------------------------------------------------------
# Crate list (dependency order — Apache/MIT only; VSAL stripped)
# ---------------------------------------------------------------------------
# Source of truth for membership: crates/*/Cargo.toml WITHOUT `publish = false`.
# If you add a new crate, append it in dependency-safe position.
# Dedup + VSAL-guard is done at runtime.
# CRATES list moved to scripts/publish-common.sh for single source of truth.
# Use PUBLISH_CRATES directly from sourced file.
CRATES=("${PUBLISH_CRATES[@]}")

# Deduplicate
declare -A SEEN
UNIQUE_CRATES=()
for crate in "${CRATES[@]}"; do
    if [[ -z "${SEEN[$crate]+_}" ]]; then
        SEEN[$crate]=1
        UNIQUE_CRATES+=("$crate")
    fi
done

# ---------------------------------------------------------------------------
# Classify error
# ---------------------------------------------------------------------------
classify_error() {
    local output="$1"

    # Priority order matters! Check specific errors BEFORE rate limit,
    # because cargo output can contain "try again" alongside real errors.
    if echo "$output" | grep -qi "already exists\|already uploaded"; then
        echo "ALREADY_PUBLISHED"
    elif echo "$output" | grep -qi "error\[E"; then
        echo "COMPILE_ERROR"
    elif echo "$output" | grep -qi "failed to verify\|failed to compile"; then
        echo "VERIFY_ERROR"
    elif echo "$output" | grep -qi "dependency .* not found\|no matching version"; then
        echo "DEP_NOT_READY"
    elif echo "$output" | grep -qi "no matching package\|does not exist"; then
        echo "NOT_FOUND"
    elif echo "$output" | grep -qi "unauthorized\|403\|forbidden\|token"; then
        echo "AUTH_ERROR"
    elif echo "$output" | grep -qi "429\|too many requests\|rate limit\|retry after"; then
        # Only classify as rate limit if no other specific error matched
        echo "RATE_LIMITED"
    elif echo "$output" | grep -qi "timeout\|timed out\|connection refused"; then
        echo "NETWORK_ERROR"
    else
        echo "UNKNOWN_ERROR"
    fi
}

# ---------------------------------------------------------------------------
# Publish one crate
# ---------------------------------------------------------------------------
publish_crate() {
    local crate=$1

    for skip in $SKIP_CRATES; do
        if [[ "$crate" == "$skip" ]]; then
            echo "  SKIP (publish = false)"
            return 0
        fi
    done

    local output
    local exit_code
    local retries=0
    local max_retries=3  # only retry rate limits, 3 times max

    while true; do
        output=$(cargo publish -p "$crate" --allow-dirty $DRY_RUN 2>&1)
        exit_code=$?

        if [[ $exit_code -eq 0 ]]; then
            echo "  ✅ PUBLISHED"
            NEW_PUBLISH_COUNT=$((${NEW_PUBLISH_COUNT:-0} + 1))
            return 0
        fi

        local error_type
        error_type=$(classify_error "$output")

        case "$error_type" in
            ALREADY_PUBLISHED)
                echo "  ⏭  Already published — skipping"
                return 0
                ;;
            RATE_LIMITED)
                retries=$((retries + 1))
                if [[ $retries -gt $max_retries ]]; then
                    echo "  ❌ RATE LIMITED — gave up after $max_retries retries"
                    echo "  --- LAST OUTPUT (verify this is really rate limit) ---"
                    echo "$output" | tail -10
                    echo "  ---"
                    echo "  Resume: cargo publish -p $crate"
                    return 1
                fi
                local wait=$((120 + retries * 120))
                echo "  ⏳ Rate limited — retry $retries/$max_retries in ${wait}s..."
                echo "  (output: $(echo "$output" | grep -i "429\|rate\|retry" | head -1))"
                sleep "$wait"
                ;;
            COMPILE_ERROR)
                echo "  ❌ COMPILE ERROR — fix code before publishing"
                echo "  ---"
                echo "$output" | grep "error\[E" | head -5
                echo "  ---"
                return 1
                ;;
            VERIFY_ERROR)
                echo "  ❌ VERIFY FAILED"
                echo "  ---"
                echo "$output" | grep -i "error\|failed" | head -5
                echo "  ---"
                return 1
                ;;
            DEP_NOT_READY)
                echo "  ❌ DEPENDENCY NOT READY on crates.io"
                echo "  ---"
                echo "$output" | grep -i "dependency\|not found\|no matching" | head -3
                echo "  ---"
                echo "  This crate's dependency hasn't propagated yet."
                echo "  Wait 5-10 min then: cargo publish -p $crate"
                return 1
                ;;
            AUTH_ERROR)
                echo "  ❌ AUTH ERROR — check cargo login / API token"
                echo "  ---"
                echo "$output" | grep -i "unauthorized\|forbidden\|token" | head -3
                echo "  ---"
                return 1
                ;;
            NETWORK_ERROR)
                retries=$((retries + 1))
                if [[ $retries -gt $max_retries ]]; then
                    echo "  ❌ NETWORK ERROR — gave up after $max_retries retries"
                    return 1
                fi
                echo "  ⏳ Network error — retry $retries/$max_retries in 60s..."
                sleep 60
                ;;
            NOT_FOUND)
                echo "  ❌ CRATE NOT FOUND in workspace"
                echo "  ---"
                echo "$output" | tail -3
                echo "  ---"
                return 1
                ;;
            *)
                echo "  ❌ UNKNOWN ERROR"
                echo "  --- FULL OUTPUT ---"
                echo "$output"
                echo "  --- END ---"
                return 1
                ;;
        esac
    done
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
TOTAL=${#UNIQUE_CRATES[@]}
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  VIL Publish — $(date)"
echo "  Crates: $TOTAL"
echo "  Log: $LOG_FILE"
echo "═══════════════════════════════════════════════════════════"

NEW_PUBLISH_COUNT=0
IDX=0
FAILED_CRATES=()
SKIPPED_CRATES=()
START_TIME=$(date +%s)

for crate in "${UNIQUE_CRATES[@]}"; do
    IDX=$((IDX + 1))
    ELAPSED=$(( $(date +%s) - START_TIME ))
    ELAPSED_MIN=$(( ELAPSED / 60 ))

    echo ""
    echo "[$IDX/$TOTAL] $crate  (${ELAPSED_MIN}m elapsed, ${NEW_PUBLISH_COUNT} published)"

    if publish_crate "$crate"; then
        :
    else
        FAILED_CRATES+=("$crate")
    fi

    # Batch cooldown
    if [[ $NEW_PUBLISH_COUNT -gt 0 && $((NEW_PUBLISH_COUNT % BATCH_SIZE)) -eq 0 && -z "$DRY_RUN" ]]; then
        echo ""
        echo "═══ Batch cooldown: ${BATCH_COOLDOWN}s ($(($BATCH_COOLDOWN/60))m) — $NEW_PUBLISH_COUNT published ═══"
        sleep "$BATCH_COOLDOWN"
    elif [[ $NEW_PUBLISH_COUNT -gt 0 && -z "$DRY_RUN" ]]; then
        # Normal wait between publishes
        local_wait=$((WAIT_SECONDS + (RANDOM % 20) - 10))
        [[ $local_wait -lt 20 ]] && local_wait=20
        echo "  Wait ${local_wait}s..."
        sleep "$local_wait"
    fi
done

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
TOTAL_TIME=$(( $(date +%s) - START_TIME ))
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  SUMMARY"
echo "═══════════════════════════════════════════════════════════"
echo "  Published:  $NEW_PUBLISH_COUNT"
echo "  Skipped:    $((TOTAL - NEW_PUBLISH_COUNT - ${#FAILED_CRATES[@]}))"
echo "  Failed:     ${#FAILED_CRATES[@]}"
echo "  Time:       $((TOTAL_TIME / 60))m $((TOTAL_TIME % 60))s"
echo ""

if [[ ${#FAILED_CRATES[@]} -gt 0 ]]; then
    echo "  FAILED CRATES:"
    for f in "${FAILED_CRATES[@]}"; do
        echo "    - $f"
    done
    echo ""
    echo "  RETRY COMMANDS:"
    for f in "${FAILED_CRATES[@]}"; do
        echo "    cargo publish -p $f"
    done
fi

echo ""
echo "  Log saved: $LOG_FILE"
echo "═══════════════════════════════════════════════════════════"
