#!/usr/bin/env bash
# =============================================================================
# Publish remaining 5 crates in correct dependency order
# =============================================================================
# Run manually:  ./scripts/publish-remaining.sh
# Each crate waits 2 min after publish for crates.io index propagation.
# =============================================================================

set -euo pipefail
cd "$(dirname "$0")/.."

CRATES=(
    vil_new_http       # L8a: deps all on crates.io (vil_rt, vil_ir, vil_json, vil_types)
    vil_sdk            # L8b: needs vil_new_http
    vil_plugin_sdk     # L8c: needs vil_server_core (already published)
    vil_server         # L9a: umbrella crate (Apache/MIT)
    vil_server_test    # L9b: needs vil_server
    vil_viz            # L-1: needed by vil_cli (publish=false removed)
    # vil_cli          # VSAL (v0.4.0+) — NOT published to crates.io
)

echo "=== Publish Remaining — $(date) ==="
echo "Crates to publish: ${CRATES[*]}"
echo ""

for crate in "${CRATES[@]}"; do
    # Check if already published
    status=$(curl -s -o /dev/null -w "%{http_code}" "https://crates.io/api/v1/crates/$crate/0.1.0")
    if [[ "$status" == "200" ]]; then
        echo "  $crate ... ⏭ already published"
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
echo "=== All remaining crates published! ==="
echo ""
echo "=== Done! All remaining crates published. ==="
