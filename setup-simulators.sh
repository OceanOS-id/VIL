#!/bin/bash
# ============================================================================
# setup-simulators.sh — VIL Simulator Setup
# ============================================================================
# Two modes:
#   docker  — Run simulators in Docker containers (simple, default)
#   binary  — Download & run native binaries (for loadtest, no Docker overhead)
#
# Usage:
#   ./setup-simulators.sh docker          # Start Docker simulators
#   ./setup-simulators.sh binary          # Download + start native binaries
#   ./setup-simulators.sh status          # Check health of all simulators
#   ./setup-simulators.sh stop            # Stop all simulators (both modes)
#
# Simulators:
#   AI Endpoint Simulator    :4545   — OpenAI/Anthropic/Ollama mock
#   Credit Data Simulator    :18081  — Core Banking NDJSON stream
# ============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$SCRIPT_DIR/simulators/bin"
PID_DIR="$SCRIPT_DIR/simulators/pid"

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
DIM='\033[2m'
NC='\033[0m'

# Simulator definitions
AI_SIM_PORT=4545
AI_SIM_BIN="$BIN_DIR/ai-endpoint-simulator"
AI_SIM_REPO="AdeOcean/ai-endpoint-simulator"

CREDIT_SIM_PORT=18081
CREDIT_SIM_BIN="$BIN_DIR/credit-data-simulator"
CREDIT_SIM_REPO="AdeOcean/credit-data-simulator"

# ── Detect platform ─────────────────────────────────────────

detect_platform() {
    local os arch
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)
    case "$arch" in
        x86_64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) arch="$arch" ;;
    esac
    echo "${os}-${arch}"
}

# ── Docker mode ──────────────────────────────────────────────

cmd_docker() {
    echo -e "${CYAN}=== Starting Simulators (Docker) ===${NC}"
    echo ""

    docker compose --profile simulators up -d 2>&1 | tail -5
    sleep 3

    echo ""
    check_health "docker"
}

# ── Binary mode ──────────────────────────────────────────────

cmd_binary() {
    echo -e "${CYAN}=== Starting Simulators (Native Binary) ===${NC}"
    echo ""

    mkdir -p "$BIN_DIR" "$PID_DIR"

    # Stop Docker simulators to avoid port conflict
    docker stop vil-ai-simulator vil-credit-simulator 2>/dev/null || true
    sleep 1

    # Download if not present
    download_if_missing "$AI_SIM_BIN" "$AI_SIM_REPO" "ai-endpoint-simulator"
    download_if_missing "$CREDIT_SIM_BIN" "$CREDIT_SIM_REPO" "credit-data-simulator"

    # Start AI simulator
    if [ -f "$AI_SIM_BIN" ]; then
        fuser -k $AI_SIM_PORT/tcp 2>/dev/null || true
        sleep 0.5
        "$AI_SIM_BIN" > /dev/null 2>&1 &
        echo $! > "$PID_DIR/ai-sim.pid"
        sleep 2
        echo -e "  ${GREEN}✓${NC} AI simulator started (PID $(cat "$PID_DIR/ai-sim.pid"))"
    else
        echo -e "  ${RED}✗${NC} AI simulator binary not found"
    fi

    # Start Credit simulator
    if [ -f "$CREDIT_SIM_BIN" ]; then
        fuser -k $CREDIT_SIM_PORT/tcp 2>/dev/null || true
        sleep 0.5
        "$CREDIT_SIM_BIN" > /dev/null 2>&1 &
        echo $! > "$PID_DIR/credit-sim.pid"
        sleep 2
        echo -e "  ${GREEN}✓${NC} Credit simulator started (PID $(cat "$PID_DIR/credit-sim.pid"))"
    else
        echo -e "  ${RED}✗${NC} Credit simulator binary not found"
    fi

    echo ""
    check_health "binary"
}

download_if_missing() {
    local bin_path="$1" repo="$2" name="$3"

    if [ -f "$bin_path" ]; then
        echo -e "  ${DIM}$name already downloaded${NC}"
        return
    fi

    echo -e "  Downloading $name..."
    local platform
    platform=$(detect_platform)

    # Try GitHub Release first
    local url="https://github.com/${repo}/releases/latest/download/${name}-${platform}"
    local http_code
    http_code=$(curl -sL -o "$bin_path" -w "%{http_code}" "$url" 2>/dev/null)

    if [ "$http_code" = "200" ] && [ -s "$bin_path" ]; then
        chmod +x "$bin_path"
        echo -e "  ${GREEN}✓${NC} Downloaded $name ($(wc -c < "$bin_path") bytes)"
        return
    fi

    rm -f "$bin_path"

    # Fallback: cargo install
    echo -e "  ${YELLOW}!${NC} GitHub release not found, trying cargo install..."
    if command -v cargo >/dev/null 2>&1; then
        cargo install "$name" --root "$BIN_DIR/.." 2>&1 | tail -3
        if [ -f "$bin_path" ]; then
            echo -e "  ${GREEN}✓${NC} Installed $name via cargo"
            return
        fi
        # cargo install puts binary in bin/ inside --root
        local cargo_bin="$BIN_DIR/../bin/$name"
        if [ -f "$cargo_bin" ]; then
            mv "$cargo_bin" "$bin_path"
            echo -e "  ${GREEN}✓${NC} Installed $name via cargo"
            return
        fi
    fi

    echo -e "  ${RED}✗${NC} Failed to download $name"
    echo -e "  ${DIM}Manual: download from https://github.com/${repo}/releases${NC}"
    echo -e "  ${DIM}Or: cargo install $name${NC}"
}

# ── Status ───────────────────────────────────────────────────

cmd_status() {
    check_health ""
}

check_health() {
    local mode="${1:-}"
    echo -e "${CYAN}=== Simulator Status ===${NC}"
    [ -n "$mode" ] && echo -e "  Mode: $mode"
    echo ""

    # AI Simulator
    local ai_code
    ai_code=$(curl -s -o /dev/null -w "%{http_code}" --max-time 3 \
        -X POST http://localhost:$AI_SIM_PORT/v1/chat/completions \
        -H "Content-Type: application/json" \
        -d '{"model":"gpt-4","messages":[{"role":"user","content":"ping"}]}' 2>/dev/null)
    if [ "$ai_code" = "200" ]; then
        echo -e "  ${GREEN}✓${NC} AI Endpoint Simulator   :$AI_SIM_PORT  — healthy"
    else
        echo -e "  ${RED}✗${NC} AI Endpoint Simulator   :$AI_SIM_PORT  — not responding (HTTP $ai_code)"
    fi

    # Credit Simulator
    local credit_health
    credit_health=$(curl -sf --max-time 3 http://localhost:$CREDIT_SIM_PORT/health 2>/dev/null)
    if echo "$credit_health" | grep -q "healthy"; then
        echo -e "  ${GREEN}✓${NC} Credit Data Simulator   :$CREDIT_SIM_PORT — healthy"
    else
        echo -e "  ${RED}✗${NC} Credit Data Simulator   :$CREDIT_SIM_PORT — not responding"
    fi

    echo ""
}

# ── Stop ─────────────────────────────────────────────────────

cmd_stop() {
    echo -e "${CYAN}=== Stopping Simulators ===${NC}"

    # Stop native binaries
    for pid_file in "$PID_DIR"/*.pid; do
        [ ! -f "$pid_file" ] && continue
        local pid
        pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null
            echo -e "  Stopped PID $pid ($(basename "$pid_file" .pid))"
        fi
        rm -f "$pid_file"
    done

    # Kill by port (catch orphans)
    fuser -k $AI_SIM_PORT/tcp 2>/dev/null || true
    fuser -k $CREDIT_SIM_PORT/tcp 2>/dev/null || true

    # Stop Docker
    docker stop vil-ai-simulator vil-credit-simulator 2>/dev/null || true

    echo -e "  ${GREEN}✓${NC} All simulators stopped"
    echo ""
}

# ── Main ─────────────────────────────────────────────────────

CMD="${1:-status}"

case "$CMD" in
    docker)  cmd_docker ;;
    binary)  cmd_binary ;;
    status)  cmd_status ;;
    stop)    cmd_stop ;;
    *)
        echo "Usage: $0 {docker|binary|status|stop}"
        echo ""
        echo "  docker  — Start simulators in Docker containers"
        echo "  binary  — Download & start native binaries (faster, for loadtest)"
        echo "  status  — Check health of all simulators"
        echo "  stop    — Stop all simulators"
        exit 1
        ;;
esac
