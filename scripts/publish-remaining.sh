#!/usr/bin/env bash
# =============================================================================
# Publish a specific subset of crates in dependency order.
# Use this after `publish-all.sh` rate-limited, or for cherry-pick releases.
# =============================================================================
# Run manually:  ./scripts/publish-remaining.sh
# Each crate waits 2 min after publish for crates.io index propagation.
#
# Override which crates to ship via env:
#   PUBLISH_ONLY="vil_sdk vil_server" ./scripts/publish-remaining.sh
# =============================================================================

set -euo pipefail
cd "$(dirname "$0")/.."

source scripts/publish-common.sh
publish_preflight_version || exit 1

# Resolve target version for already-published check
TARGET_VERSION="${VIL_PUBLISH_VERSION:-0.4.0}"

# Default = curated dependency-tail; override via PUBLISH_ONLY env
if [[ -n "${PUBLISH_ONLY:-}" ]]; then
    read -r -a CRATES <<<"$PUBLISH_ONLY"
else
    CRATES=(
        vil_new_http       # L8a: deps on vil_rt, vil_ir, vil_json, vil_types
        vil_sdk            # L8b: needs vil_new_http
        vil_plugin_sdk     # L8c: needs vil_server_core
        vil_server         # L9a: umbrella crate (Apache/MIT, not VSAL)
        vil_server_test    # L9b: needs vil_server
        vil_viz            # L-1
        # vil_cli is VSAL (v0.4.0+) — NOT on crates.io
    )
fi

echo "=== Publish Remaining (v${TARGET_VERSION}) — $(date) ==="
echo "Crates: ${CRATES[*]}"
echo ""

for crate in "${CRATES[@]}"; do
    # VSAL / legacy guard — refuse to publish unpublishable crates
    for skip in $PUBLISH_SKIP_CRATES; do
        if [[ "$crate" == "$skip" ]]; then
            echo "  $crate ... ⏭ skipped (VSAL or publish=false)"
            continue 2
        fi
    done

    # Check if already published at TARGET version
    status=$(curl -s -o /dev/null -w "%{http_code}" \
      "https://crates.io/api/v1/crates/$crate/$TARGET_VERSION")
    if [[ "$status" == "200" ]]; then
        echo "  $crate ... ⏭ already published at v${TARGET_VERSION}"
        continue
    fi

    echo -n "  Publishing $crate ... "
    output=$(cargo publish -p "$crate" --allow-dirty 2>&1)

    if echo "$output" | grep -q "Published\|Uploaded"; then
        echo "✅"
    elif echo "$output" | grep -q "already exists"; then
        echo "⏭ (exists)"
        continue
    elif echo "$output" | grep -q "429\|Too Many Requests"; then
        echo "⏳ RATE LIMITED"
        echo "  Wait 10 minutes then re-run this script."
        exit 1
    else
        echo "❌"
        echo "$output" | tail -5
        echo ""
        echo "  Fix the error above, then re-run this script."
        exit 1
    fi

    # Wait for crates.io index to propagate before next publish
    echo "  Waiting 2 min for index propagation..."
    sleep 120
done

echo ""
echo "=== All selected crates published at v${TARGET_VERSION}. ==="
