#!/usr/bin/env bash
# =============================================================================
# build.sh — vil-ai-gw-demo
# =============================================================================
# Build the example in an isolated temporary workspace that symlinks only the
# minimal directories required (example + crates). This avoids cargo attempting
# to load missing workspace members while still resolving path dependencies.
#
# Behavior:
#  - Creates a temporary workspace directory
#  - Symlinks:
#      <tmp>/examples/001-vil-ai-gw-demo -> real example dir
#      <tmp>/crates                      -> real workspace crates dir (if exists)
#  - Writes a temporary Cargo.toml workspace pointing to the example member
#  - Runs cargo build using --manifest-path against the temp workspace
#  - Uses --target-dir to put build artifacts under the example's target/
#  - Cleans up temp dir on exit
#
# Usage:
#   ./build.sh              # dev build (default)
#   ./build.sh release      # release build (optimized)
#   ./build.sh --workspace /path/to/root release
#   ./build.sh --help
#
# Notes:
#  - This script prefers manifest-based building so it will not require the
#    entire (original) workspace members to be present.
#  - If your example's path dependencies point to other locations, ensure those
#    directories are reachable from the example path (we attempt to symlink the
#    top-level 'crates' directory automatically).
# =============================================================================

set -euo pipefail

# --- Helpers -----------------------------------------------------------------
err() { printf "❌ %s\n" "$*" >&2; }
info() { printf "ℹ️  %s\n" "$*"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLE_DIR="$(cd "${SCRIPT_DIR}" && pwd)"
EXAMPLE_MANIFEST="${EXAMPLE_DIR}/Cargo.toml"

# defaults
PROFILE="dev"
OVERRIDE_WS=""

print_help() {
cat <<'USAGE'
VIL AI Gateway Demo - Build Script

Usage:
  build.sh [dev|release]
  build.sh --workspace /path/to/workspace [dev|release]
  build.sh --help

Options:
  --workspace PATH   Optional: workspace root to use for informational symlinks
  dev                Build in development mode (default)
  release            Build in release mode (optimized)

Behavior:
  - Builds only the example by creating a temporary workspace manifest that
    references the example member. The script symlinks the example and the
    'crates' directory (if present) into the temporary workspace so path
    dependencies like "../../crates/vil_sdk" resolve properly.
  - Artifacts are written to the example's target directory (examples/001-vil-ai-gw-demo/target)
USAGE
}

# --- Parse args --------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        dev) PROFILE="dev"; shift ;;
        release) PROFILE="release"; shift ;;
        --workspace)
            if [[ -z "${2:-}" ]]; then
                err "--workspace requires a path"
                exit 1
            fi
            OVERRIDE_WS="$2"
            shift 2
            ;;
        -h|--help) print_help; exit 0 ;;
        *) err "Unknown argument: $1"; print_help; exit 1 ;;
    esac
done

# sanity
if [[ ! -f "$EXAMPLE_MANIFEST" ]]; then
    err "Example manifest not found: $EXAMPLE_MANIFEST"
    exit 1
fi

# Determine top-level workspace root (for symlinking crates/) by walking up
# from the example directory unless user provided an override.
find_workspace_root() {
    local d="$1"
    while [[ -n "$d" && "$d" != "/" ]]; do
        if [[ -f "$d/Cargo.toml" ]]; then
            printf '%s' "$d"
            return 0
        fi
        d="$(dirname "$d")"
    done
    return 1
}

if [[ -n "$OVERRIDE_WS" ]]; then
    WORKSPACE_ROOT="$(cd "$OVERRIDE_WS" 2>/dev/null && pwd || true)"
    if [[ -z "$WORKSPACE_ROOT" || ! -f "${WORKSPACE_ROOT}/Cargo.toml" ]]; then
        err "Provided workspace does not contain Cargo.toml: $OVERRIDE_WS"
        exit 1
    fi
else
    if ! WORKSPACE_ROOT="$(find_workspace_root "$EXAMPLE_DIR/../..")"; then
        if ! WORKSPACE_ROOT="$(find_workspace_root "$EXAMPLE_DIR")"; then
            # fallback: use example dir as workspace root (we will still build via manifest)
            WORKSPACE_ROOT="$EXAMPLE_DIR"
        fi
    fi
fi

info "Building from workspace: ${WORKSPACE_ROOT}"

# Build using the real workspace root so all workspace.package / workspace.dependencies
# inheritance in crates/ resolves correctly.
info "Building example via workspace manifest..."
if [[ "$PROFILE" == "release" ]]; then
    info "Profile: release"
    CARGO_PROFILE_RELEASE_LTO=false cargo build \
        --manifest-path "${WORKSPACE_ROOT}/Cargo.toml" \
        -p vil-ai-gw-demo --release
    RETVAL=$?
else
    info "Profile: dev"
    cargo build \
        --manifest-path "${WORKSPACE_ROOT}/Cargo.toml" \
        -p vil-ai-gw-demo
    RETVAL=$?
fi

if [[ $RETVAL -ne 0 ]]; then
    err "Cargo build failed (exit $RETVAL)"
    exit $RETVAL
fi

# Compute binary path inside workspace target/
if [[ "$PROFILE" == "release" ]]; then
    BINARY_PATH="${WORKSPACE_ROOT}/target/release/vil-ai-gw-demo"
else
    BINARY_PATH="${WORKSPACE_ROOT}/target/debug/vil-ai-gw-demo"
fi

if [[ -f "$BINARY_PATH" ]]; then
    info "Build successful: $BINARY_PATH"
    if command -v du >/dev/null 2>&1; then
        printf "Binary size: "; du -h "$BINARY_PATH" | awk '{print $1}'
    fi
    echo
    info "Next: run the example"
    info "  ./run.sh             # dev"
    info "  ./run.sh release     # release"
    exit 0
else
    err "Build finished but binary not found at expected path: $BINARY_PATH"
    exit 1
fi
