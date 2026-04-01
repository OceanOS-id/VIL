# HANDOFF: SQA Testing — VIL CLI, Templates, Examples & Website

**Date:** 2026-03-31
**From:** Engineering
**To:** SQA Team
**Priority:** High
**Scope:** VIL CLI (v0.1.18), 88 examples, vil init templates, website quickstart, Vastar Bench

---

## 0. Test Suite Setup

Clone the automated test suite first:

```bash
git clone https://github.com/OceanOS-id/vil-testsuite.git
cd vil-testsuite
```

Run all automated tests:

```bash
# Run everything
./run.sh

# Run specific spec
./run.sh cli          # CLI commands only
./run.sh init         # All 88 template init tests
./run.sh bench        # Vastar Bench output format
./run.sh observer     # Observer API endpoints
./run.sh runtime      # Build + run + curl all examples
./run.sh edge_cases   # Edge case scenarios

# List available specs
./run.sh --list

# Run runtime tests including simulator-dependent examples
RUNTIME_ALL=1 ./run.sh runtime
```

Test suite auto-discovers all 88 examples from `template-index.json`. When new examples are added, pull latest and re-run — no test code changes needed.

**Update test suite:**
```bash
cd vil-testsuite
git pull
```

**Update template index (after new examples added to VIL):**
```bash
cd vil  # VIL repo
python3 scripts/generate-all-templates.py         # generate template.toml for new examples
python3 scripts/generate-all-templates.py --index > template-index.json  # regenerate index
```

---

## 1. Prerequisites

Install these on test machine before starting:

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# VIL CLI
cargo install vil_cli
vil --help  # verify: should show 30+ subcommands

# Vastar Bench
cargo install vastar
vastar --version  # verify: v0.1.8

# AI Endpoint Simulator
cargo install ai-endpoint-simulator
ai-endpoint-simulator &  # runs on port 4545
```

---

## 2. Test: `vil templates`

### 2.1 List templates (online)

```bash
vil templates
```

**Expected:**
- Fetches from GitHub
- Shows 10 templates with ID, TITLE, DESCRIPTION
- Sync status: OK (if previously synced) or -- (not synced)
- Usage hint at bottom

### 2.2 Sync templates

```bash
vil templates --sync
```

**Expected:**
- Downloads all 10 templates to `~/vastar/vil/examples/`
- Shows progress per template
- "DONE Synced 10 templates, N files"

### 2.3 Verify sync

```bash
vil templates
```

**Expected:** All 10 show "OK" status

### 2.4 Offline test

```bash
# Disconnect internet / block GitHub
vil templates
```

**Expected:** Falls back to local VASTAR_HOME, still shows templates

---

## 3. Test: `vil init` (Direct Arguments)

Test each template with explicit arguments. Each test:
1. Init project
2. Verify files generated
3. Verify package name replaced
4. Build (where applicable)

### 3.1 ai-gateway (VilApp pattern)

```bash
rm -rf ~/vastar/test-ai-gw
vil init test-ai-gw --template ai-gateway --lang rust --port 3080

# Verify
cat ~/vastar/test-ai-gw/Cargo.toml | grep "name"
# Expected: name = "test-ai-gw"

cat ~/vastar/test-ai-gw/src/main.rs | grep "port"
# Expected: .port(3080)

# Build
cd ~/vastar/test-ai-gw && cargo build --release
# Expected: builds without error
```

### 3.2 blank (REST API Service)

```bash
rm -rf /tmp/test-blank
vil init /tmp/test-blank --template blank --port 9090

cat /tmp/test-blank/Cargo.toml | grep "name"
# Expected: name = "test-blank"

cat /tmp/test-blank/src/main.rs | grep "port\|transform\|echo\|health"
# Expected: endpoints visible
```

### 3.3 rest-crud

```bash
rm -rf /tmp/test-crud
vil init /tmp/test-crud --template rest-crud

cat /tmp/test-crud/Cargo.toml | grep "name"
# Expected: name = "test-crud"

ls /tmp/test-crud/
# Expected: Cargo.toml, src/main.rs, app.vil.yaml, README.md, vil-server.yaml
```

### 3.4 websocket-chat

```bash
rm -rf /tmp/test-ws
vil init /tmp/test-ws --template websocket-chat

cat /tmp/test-ws/Cargo.toml | grep "name"
# Expected: name = "test-ws"
```

### 3.5 multi-model-router

```bash
rm -rf /tmp/test-router
vil init /tmp/test-router --template multi-model-router --port 3090

cat /tmp/test-router/Cargo.toml | grep "name"
cat /tmp/test-router/src/main.rs | grep "UPSTREAM_URL"
# Expected: name = "test-router", upstream URL present
```

### 3.6 rag-pipeline

```bash
rm -rf /tmp/test-rag
vil init /tmp/test-rag --template rag-pipeline

cat /tmp/test-rag/Cargo.toml | grep "name"
# Expected: name = "test-rag"
```

### 3.7 agent

```bash
rm -rf /tmp/test-agent
vil init /tmp/test-agent --template agent

cat /tmp/test-agent/Cargo.toml | grep "name"
# Expected: name = "test-agent"
```

### 3.8 wasm-faas

```bash
rm -rf /tmp/test-wasm
vil init /tmp/test-wasm --template wasm-faas

cat /tmp/test-wasm/Cargo.toml | grep "name"
# Expected: name = "test-wasm"
```

### 3.9 iot-gateway

```bash
rm -rf /tmp/test-iot
vil init /tmp/test-iot --template iot-gateway

cat /tmp/test-iot/Cargo.toml | grep "name"
# Expected: name = "test-iot"
```

### 3.10 observer-demo

```bash
rm -rf /tmp/test-obs
vil init /tmp/test-obs --template observer-demo

cat /tmp/test-obs/Cargo.toml | grep "name"
# Expected: name = "test-obs"
```

---

## 4. Test: `vil init` (Interactive Wizard)

### 4.1 Full wizard flow

```bash
vil init
```

**Interaction sequence:**
1. Project name → type: `wizard-test`
2. Language → type: `1` (Rust)
3. Template list → **verify 10 templates from GitHub, not hardcoded 12**
4. Template → type: `1` (AI Gateway)
5. Token → press Enter (default: shm)
6. Port → press Enter (default: 3081)
7. Upstream → press Enter (default)

**Expected:**
- "FETCH Downloading template files..."
- "DONE Project 'wizard-test' created!"
- Files at `~/vastar/wizard-test/`

### 4.2 Wizard with non-Rust language

```bash
vil init
```

Select language: `2` (Python)

**Expected:** Falls back to legacy codegen (not example-based). Generates Python SDK file.

### 4.3 Wizard with different template numbers

Test selecting template by number (1-10) and by name (e.g., `rest-crud`).

---

## 5. Test: `vil init` Edge Cases

### 5.1 Directory already exists

```bash
mkdir -p /tmp/test-exists
vil init /tmp/test-exists --template blank
```

**Expected:** Error message about directory existing

### 5.2 Invalid template name

```bash
vil init /tmp/test-invalid --template nonexistent
```

**Expected:** Error or fallback to legacy. Should not crash.

### 5.3 No internet (offline, synced)

```bash
vil templates --sync   # sync first
# Disconnect internet
vil init /tmp/test-offline --template ai-gateway
```

**Expected:** Uses local VASTAR_HOME templates, succeeds

### 5.4 No internet (offline, NOT synced)

```bash
rm -rf ~/vastar/vil/examples/  # remove synced templates
# Disconnect internet
vil init /tmp/test-offline2 --template ai-gateway
```

**Expected:** Falls back to legacy codegen, still generates project

### 5.5 Path with spaces

```bash
vil init "/tmp/test with spaces" --template blank
```

**Expected:** Creates directory with spaces, files generated correctly

---

## 6. Test: Website Quickstart Flow

Replicate exactly what user sees at `vastar.id/products/vil`.

### 6.1 Step 0: Preparation

```bash
# Install Rust (skip if already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install vastar
curl -sSf https://raw.githubusercontent.com/Vastar-AI/vastar/main/install.sh | sh
vastar --version
# Expected: vastar 0.1.x

# Alternative: hey
# mkdir -p ~/.local/bin
# curl -sL https://hey-release.s3.us-east-2.amazonaws.com/hey_linux_amd64 -o ~/.local/bin/hey
# chmod +x ~/.local/bin/hey
```

### 6.2 Step 1: Create project

```bash
cargo install vil_cli ai-endpoint-simulator

# List templates
vil templates

# Create project
vil init my-gateway --lang rust --template ai-gateway

# Or use wizard
# vil init
```

**Verify:**
- `~/vastar/my-gateway/Cargo.toml` exists
- `name = "my-gateway"`
- `src/main.rs` has VilApp code

### 6.3 Step 2: Build & Run

```bash
ai-endpoint-simulator &
cd ~/vastar/my-gateway
cargo run --release
```

**Expected:**
- Simulator on port 4545
- Gateway starts on port 3081
- Shows curl/hey/vastar instructions

### 6.4 Step 3: Test

```bash
# In another terminal

# Curl test
curl -s -X POST http://localhost:3081/api/gw/trigger \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"hello"}'
# Expected: JSON response with AI content

# Vastar benchmark
vastar -m POST -H 'Content-Type: application/json' \
  -d '{"prompt":"bench"}' -c 300 -n 3000 \
  http://localhost:3081/api/gw/trigger
# Expected: Summary, histogram, SLO, Insight

# Hey benchmark (if installed)
hey -m POST -H 'Content-Type: application/json' \
  -d '{"prompt":"bench"}' -c 300 -n 3000 \
  http://localhost:3081/api/gw/trigger
# Expected: Summary with RPS
```

### 6.5 Step 4: Observer Dashboard

```bash
OBSERVER=1 cargo run --release
```

Open browser: `http://localhost:3081/_vil/dashboard/`

**Verify:**
- Dashboard loads (dark theme)
- Sidebar: Dashboard + Topology icons
- Throughput gauges update live
- Routes table shows POST /api/gw/trigger
- Right sidebar: SLO Budget, Alerts, System, Config
- Run vastar bench → dashboard shows live metrics
- Click Topology → shows service graph

---

## 7. Test: Vastar Bench Output

### 7.1 Basic output

```bash
vastar -n 1000 -c 100 http://localhost:3081/api/gw/trigger
```

**Verify:**
- Summary section (Total, Slowest, Fastest, Average, RPS)
- Response time distribution (p10 through p99.99)
- Key percentiles highlighted with (ms): p50, p95, p99, p99.9
- Response time histogram (11 buckets, colored bars ■)
- SLO legend (4 rows × 3 columns)
- SLO note: "SLO levels are relative to this run's..."
- Status code distribution (colored: green=200, red=5xx)
- Details (req write, resp wait, resp read)
- Insight (spread, tail, outlier)
- Newline after every section header

### 7.2 Error scenario

```bash
# Stop the gateway, then:
vastar -n 100 -c 10 http://localhost:9999/
```

**Expected:**
- "Errors: 10 total"
- "All 10 requests failed. Is the target running?"
- No histogram, no Insight

### 7.3 502 error scenario

```bash
# Run gateway, increase concurrency until 502s appear
vastar -n 10000 -c 600 -m POST -H 'Content-Type: application/json' \
  -d '{"prompt":"bench"}' http://localhost:3081/api/gw/trigger
```

**Expected:**
- Status codes: [200] N, [502] M -- Bad Gateway
- Insight: Error rate X% -- CRITICAL (502xM)
- Colored: 200=green, 502=dark red

### 7.4 Progress bar (run in real terminal, not piped)

```bash
vastar -n 50000 -c 300 -m POST -T "application/json" \
  -d '{"prompt":"bench"}' http://localhost:4545/v1/chat/completions
```

**Expected:**
- Live progress: colored ■ bar gradient
- RPS and Avg colored green
- Updates 10 FPS
- No progress when piped: `vastar ... 2>/dev/null`

---

## 8. Test: Observer API Endpoints

With gateway running (`OBSERVER=1`):

```bash
# Prometheus
curl -s http://localhost:3081/_vil/metrics | head -10
# Expected: Prometheus text format, vil_uptime_seconds, vil_requests_total

# SLO
curl -s http://localhost:3081/_vil/api/slo | python3 -m json.tool
# Expected: target_pct, current_pct, budget_remaining, status

# Alerts
curl -s http://localhost:3081/_vil/api/alerts | python3 -m json.tool
# Expected: alerts array (empty if no issues)

# Routes
curl -s http://localhost:3081/_vil/api/routes | python3 -m json.tool
# Expected: route info with method, path, latency percentiles

# System
curl -s http://localhost:3081/_vil/api/system | python3 -m json.tool
# Expected: pid, cpu_count, memory_rss_kb, uptime_secs

# Health
curl -s http://localhost:3081/_vil/api/health
# Expected: {"status":"healthy",...}
```

---

## 9. Test Matrix Summary

| Test Area | Cases | Priority |
|---|---|---|
| `vil templates` | list, sync, offline fallback | High |
| `vil init` (10 templates) | each template, file verification, name replacement | High |
| `vil init` wizard | full flow, language fallback, template selection | High |
| `vil init` edge cases | exists, invalid, offline, spaces | Medium |
| Website quickstart | Step 0-4, exact copy-paste from website | Critical |
| Vastar Bench output | all sections, errors, progress bar | High |
| Observer dashboard | UI, API endpoints, SLO, alerts | High |
| Observer Prometheus | scrape format, per-route metrics | Medium |

---

## 10. Known Issues

1. **Package name with full path** — `vil init /tmp/test-x --template blank` may set name to `/tmp/test-x` instead of `test-x`. Fix deployed in v0.1.18 but verify.
2. **Wizard template list** — should show 10 dynamic templates from GitHub, not 12 hardcoded. Verify wizard fetches from GitHub.
3. **SLO Budget "exhausted" at 0 requests** — fixed, should show "healthy" when no traffic. Verify.
4. **Vastar progress bar** — only visible when stderr is a TTY. When piped (`2>/dev/null`), no progress output.
5. **Observer alert logging** — alerts print to stderr with `[VIL ALERT]` prefix. Check server terminal output during high-error bench.

---

## 11. Cleanup After Testing

```bash
rm -rf ~/vastar/test-*
rm -rf ~/vastar/my-gateway
rm -rf ~/vastar/wizard-test
rm -rf /tmp/test-*
pkill ai-endpoint-simulator
```

---

**Contact:** Engineering team for any blockers or unexpected behavior.
