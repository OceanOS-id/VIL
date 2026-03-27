#!/bin/bash
# =============================================================================
# run.sh — vil-ai-gw-demo
# =============================================================================
# Run the VIL AI Gateway example from workspace source.
# Uses relative paths (works on any system/user).
#
# Usage:
#   From example directory:     ./run.sh [dev|release] [options]
#   From workspace root:        ./examples/001-vil-ai-gw-demo/run.sh [...]
#   Direct cargo:               cargo run -p vil-ai-gw-demo
#
# Options:
#   dev                 Run in dev mode (default, faster startup)
#   release             Run in release mode (optimized, slower startup)
#   --log-level LEVEL   Set RUST_LOG level (trace|debug|info|warn|error)
#   --help              Show this help message
#
# Examples:
#   ./run.sh                                    # Run dev, info log level
#   ./run.sh release                            # Run release, info log level
#   ./run.sh dev --log-level debug              # Dev with debug logging
#   ./run.sh release --log-level trace          # Release with trace logging
# =============================================================================

set -e

# ─── Determine workspace root using relative path ──────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Verify we are inside a workspace by checking for Cargo.toml at the computed root.
# Do not enforce a specific workspace name to allow flexible workspace layouts.
if [ ! -f "${WORKSPACE}/Cargo.toml" ]; then
    echo "❌ Error: Cargo.toml not found at ${WORKSPACE}"
    echo "   Expected workspace root at: ${WORKSPACE}"
    exit 1
fi

# ─── Default values ───────────────────────────────────────────────────────────
PROFILE="dev"
LOG_LEVEL="info"
CLEANUP_PORTS=true

# ─── Parse arguments ──────────────────────────────────────────────────────────
while [[ "$#" -gt 0 ]]; do
    case "$1" in
        dev|development)
            PROFILE="dev"
            shift
            ;;
        release|prod|production)
            PROFILE="release"
            shift
            ;;
        --log-level|-l)
            LOG_LEVEL="$2"
            shift 2
            ;;
        --no-cleanup)
            CLEANUP_PORTS=false
            shift
            ;;
        --help|-h)
            cat << 'EOF'
VIL AI Gateway Demo - Run Script

Usage: run.sh [profile] [options]

Profiles:
  dev       Run in development mode (default, faster startup)
  release   Run in release mode (slower startup, optimized binary)

Options:
  --log-level LEVEL   Set RUST_LOG level (default: info)
                      Valid: trace, debug, info, warn, error
  --no-cleanup        Don't kill existing processes on ports
  --help              Show this help message

Examples:
  run.sh                          # Dev mode, info logging
  run.sh release                  # Release mode, info logging
  run.sh dev --log-level debug    # Dev mode, debug logging
  run.sh --log-level trace        # Dev mode, trace logging

Port Usage:
  3080  - Webhook trigger endpoint
  4545  - AI endpoint simulator (external)

Environment Variables:
  RUST_LOG  Override logging level (examples: debug, vil_rt=trace)
EOF
            exit 0
            ;;
        *)
            echo "❌ Unknown option: $1"
            echo "   Run '$0 --help' for usage information"
            exit 1
            ;;
    esac
done

# ─── Validate log level ───────────────────────────────────────────────────────
case "${LOG_LEVEL}" in
    trace|debug|info|warn|error)
        ;;
    *)
        echo "❌ Invalid log level: ${LOG_LEVEL}"
        echo "   Valid levels: trace, debug, info, warn, error"
        exit 1
        ;;
esac

# ─── Cleanup existing processes ────────────────────────────────────────────────
if [ "${CLEANUP_PORTS}" == "true" ]; then
    echo "🧹 Cleaning up ports 3080, 3081..."

    # Kill processes on port 3080 (webhook gateway)
    if command -v fuser &> /dev/null; then
        fuser -k 3080/tcp 2>/dev/null || true
        fuser -k 3081/tcp 2>/dev/null || true
    elif command -v lsof &> /dev/null; then
        lsof -ti:3080 | xargs kill -9 2>/dev/null || true
        lsof -ti:3081 | xargs kill -9 2>/dev/null || true
    else
        echo "   ⚠️  Warning: fuser/lsof not found, skipping port cleanup"
    fi

    sleep 1
fi

# ─── Verify binary exists ─────────────────────────────────────────────────────
if [ "${PROFILE}" == "release" ]; then
    BINARY_PATH="${WORKSPACE}/target/release/vil-ai-gw-demo"
else
    BINARY_PATH="${WORKSPACE}/target/debug/vil-ai-gw-demo"
fi

if [ ! -f "${BINARY_PATH}" ]; then
    echo "❌ Error: Binary not found at ${BINARY_PATH}"
    echo ""
    echo "💡 Build the project first:"
    echo "   ./examples/001-vil-ai-gw-demo/build.sh ${PROFILE}"
    echo "   OR: cargo build -p vil-ai-gw-demo${PROFILE:+ --${PROFILE}}"
    exit 1
fi

# ─── Setup environment ─────────────────────────────────────────────────────────
cd "${WORKSPACE}"

# Build RUST_LOG with appropriate modules
export RUST_LOG="vil_ai_gw_demo=${LOG_LEVEL},vil_new_http=${LOG_LEVEL},vil_rt=${LOG_LEVEL},${LOG_LEVEL}"

# ─── Display startup information ───────────────────────────────────────────────
echo ""
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║      VIL AI Gateway Demo - Starting                        ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""
echo "📊 Configuration:"
echo "   Mode: ${PROFILE}"
echo "   Log Level: ${LOG_LEVEL}"
echo "   Workspace: ${WORKSPACE}"
echo "   Binary: ${BINARY_PATH}"
echo ""
echo "🔗 Endpoints:"
echo "   Webhook Gateway: http://localhost:3080/trigger"
echo "   AI Simulator: http://localhost:4545/v1/chat/completions"
echo ""
echo "⚙️  Environment:"
echo "   RUST_LOG=${RUST_LOG}"
echo ""
echo "📝 Press Ctrl+C to stop"
echo ""

# ─── Run the application ──────────────────────────────────────────────────────
if [ "${PROFILE}" == "release" ]; then
    echo "🚀 Running vil-ai-gw-demo [RELEASE]..."
    echo ""
else
    echo "🛠️  Running vil-ai-gw-demo [DEV]..."
    echo ""
fi
"${BINARY_PATH}"

EXIT_CODE=$?

# ─── Display shutdown information ──────────────────────────────────────────────
echo ""
if [ ${EXIT_CODE} -eq 0 ]; then
    echo "✅ Application stopped gracefully"
else
    echo "❌ Application exited with code ${EXIT_CODE}"
fi

echo ""
echo "💡 Quick Reference:"
echo ""
echo "   Single Request (curl):"
echo "   curl -N -X POST -H 'Content-Type: application/json' \\"
echo "     -d '{\"prompt\": \"test\"}' \\"
echo "     http://localhost:3080/trigger"
echo ""
echo "   Load Test (oha - 400 concurrent, 4000 total):"
echo "   oha -m POST -H 'Content-Type: application/json' \\"
echo "     -d '{\"prompt\": \"benchmark\"}' \\"
echo "     -c 400 -n 4000 \\"
echo "     http://localhost:3080/trigger"
echo ""
echo "   Build optimized binary:"
echo "   ./examples/001-vil-ai-gw-demo/build.sh release"
echo ""

exit ${EXIT_CODE}
