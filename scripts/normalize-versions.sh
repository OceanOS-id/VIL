#!/usr/bin/env bash
# =============================================================================
# VIL Crate Version Normalizer
# =============================================================================
# Normalizes ALL crate versions to use workspace version.
#
# What it does:
#   1. Sets workspace version in root Cargo.toml to TARGET_VERSION
#   2. Converts all hardcoded `version = "x.y.z"` in crates/ to
#      `version.workspace = true`
#   3. Reports what changed
#
# Usage:
#   ./scripts/normalize-versions.sh              # default: 0.2.0
#   ./scripts/normalize-versions.sh 0.3.0        # custom version
#   ./scripts/normalize-versions.sh --dry-run    # preview changes only
# =============================================================================

set -euo pipefail

TARGET_VERSION="${1:-0.2.0}"
DRY_RUN=false

if [[ "$TARGET_VERSION" == "--dry-run" ]]; then
    DRY_RUN=true
    TARGET_VERSION="${2:-0.2.0}"
fi

cd "$(dirname "$0")/.."

echo "============================================"
echo "  VIL Version Normalizer"
echo "============================================"
echo "  Target version: $TARGET_VERSION"
echo "  Dry run: $DRY_RUN"
echo "============================================"
echo ""

# ---------------------------------------------------------------------------
# Step 1: Update workspace version in root Cargo.toml
# ---------------------------------------------------------------------------
echo "=== Step 1: Root Cargo.toml workspace version ==="
CURRENT_WS=$(grep "^version" Cargo.toml | head -1 | sed 's/version = "//;s/"//')
echo "  Current: $CURRENT_WS"
echo "  Target:  $TARGET_VERSION"

if [[ "$DRY_RUN" == "false" ]]; then
    sed -i "0,/^version = .*/s//version = \"$TARGET_VERSION\"/" Cargo.toml
    echo "  ✅ Updated"
else
    echo "  (dry-run, no change)"
fi
echo ""

# ---------------------------------------------------------------------------
# Step 2: Find crates with hardcoded versions (not using workspace)
# ---------------------------------------------------------------------------
echo "=== Step 2: Crates with hardcoded versions ==="
CHANGED=0
ALREADY_OK=0
TOTAL=0

for toml in crates/*/Cargo.toml; do
    TOTAL=$((TOTAL + 1))
    CRATE_NAME=$(basename "$(dirname "$toml")")

    # Check if already using workspace version
    if grep -q "^version.workspace = true" "$toml" 2>/dev/null; then
        ALREADY_OK=$((ALREADY_OK + 1))
        continue
    fi

    # Get current hardcoded version
    CURRENT=$(grep "^version = " "$toml" | head -1 | sed 's/version = "//;s/"//')
    if [[ -z "$CURRENT" ]]; then
        continue
    fi

    echo "  $CRATE_NAME: $CURRENT → version.workspace = true"
    CHANGED=$((CHANGED + 1))

    if [[ "$DRY_RUN" == "false" ]]; then
        # Replace first occurrence of version = "..." with version.workspace = true
        sed -i "0,/^version = \".*\"/s//version.workspace = true/" "$toml"
    fi
done

# ---------------------------------------------------------------------------
# Step 3: Fix inter-crate version references (vil_xxx = { version = "0.1", ... })
# ---------------------------------------------------------------------------
echo "=== Step 3: Inter-crate version references ==="
TARGET_MAJOR_MINOR=$(echo "$TARGET_VERSION" | sed 's/\.[0-9]*$//')  # e.g. 0.2.0 → 0.2
REFS_FIXED=0

for toml in crates/*/Cargo.toml; do
    CRATE_NAME=$(basename "$(dirname "$toml")")

    # Count vil_ deps with old version strings
    OLD_REFS=$(grep -c 'vil_.*version = "0\.1"' "$toml" 2>/dev/null || true)
    if [[ "$OLD_REFS" -gt 0 ]]; then
        echo "  $CRATE_NAME: $OLD_REFS refs (0.1 → $TARGET_MAJOR_MINOR)"
        REFS_FIXED=$((REFS_FIXED + OLD_REFS))

        if [[ "$DRY_RUN" == "false" ]]; then
            sed -i "s/\(vil_[a-z_]* = { version = \)\"0\.1\"/\1\"$TARGET_MAJOR_MINOR\"/g" "$toml"
        fi
    fi

    # Also fix version = "0.1.x" patterns
    OLD_REFS2=$(grep -c 'vil_.*version = "0\.1\.[0-9]*"' "$toml" 2>/dev/null || true)
    if [[ "$OLD_REFS2" -gt 0 ]]; then
        echo "  $CRATE_NAME: $OLD_REFS2 refs (0.1.x → $TARGET_MAJOR_MINOR)"
        REFS_FIXED=$((REFS_FIXED + OLD_REFS2))

        if [[ "$DRY_RUN" == "false" ]]; then
            sed -i "s/\(vil_[a-z_]* = { version = \)\"0\.1\.[0-9]*\"/\1\"$TARGET_MAJOR_MINOR\"/g" "$toml"
        fi
    fi
done

echo "  Total refs fixed: $REFS_FIXED"
echo ""

echo "============================================"
echo "  Summary"
echo "============================================"
echo "  Total crates:       $TOTAL"
echo "  Already OK:         $ALREADY_OK"
echo "  Versions changed:   $CHANGED"
echo "  Dep refs fixed:     $REFS_FIXED"
echo "  Target version:     $TARGET_VERSION"
echo "============================================"

if [[ "$DRY_RUN" == "true" ]]; then
    echo ""
    echo "  This was a dry run. No files were modified."
    echo "  Run without --dry-run to apply changes."
fi

echo ""
echo "Next steps:"
echo "  1. cargo check                    # verify all crates compile"
echo "  2. ./scripts/publish-all.sh       # publish to crates.io"
