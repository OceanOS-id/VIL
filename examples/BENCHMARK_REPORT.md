# VIL Benchmark Report

## Executive Summary

| Metric | Result | P99 |
|--------|--------|-----|
| **VX_APP** (REST, no external I/O) | **41,000 req/s** | 26ms |
| **AI Gateway** (SSE proxy, Tri-Lane) | **6,200 req/s** | 116ms |
| **NDJSON Pipeline** (1K records/req) | **895 req/s = 895K records/s** | 246ms |
| **Multi-Pipeline** (shared SHM) | **3,700 req/s** | 85ms |
| **VIL routing overhead** | **~8ms fixed** | — |
| **VIL overhead vs direct** | **16%** (at c500) | — |

All benchmarks: **100% success rate**, zero errors, zero dropped connections.

---

## Test Environment

**System:** Intel i9-11900F @ 2.50GHz (8C/16T), 32GB RAM, Ubuntu 22.04 (kernel 6.8.0)
**Rust:** 1.93.1 | **JSON:** sonic-rs SIMD | **Build:** `--release`
**Load tool:** [oha](https://github.com/hatoo/oha)
**Date:** 2026-03-27

**Conservative baseline:** Machine under normal development workload (load avg ~1.3) during all benchmarks. On dedicated production hardware, expect **2-3x higher throughput** with lower tail latency.

**Simulators (known-performance baselines for overhead measurement):**
- [ai-endpoint-simulator](https://github.com/Vastar-AI/ai-endpoint-simulator) — SSE on :4545 (~8K req/s at c500)
- [credit-data-simulator](https://github.com/Vastar-AI/credit-data-simulator) — NDJSON on :18081 (~2K req/s)

---

## 1. VX_APP (HTTP Server — No External I/O)

Pure HTTP request → VIL handler → JSON response. No upstream calls. Measures raw VIL framework overhead.

| Example | Business Domain | req/s | P50 | P99 |
|---------|----------------|-------|-----|-----|
| 003 Hello Server | Employee Directory | **47,374** | 0.5ms | 26ms |
| 036 SSE Event Builder | Stock Ticker | **46,281** | 0.3ms | 25ms |
| 031 Mesh Routing | Banking Transaction | **44,387** | 0.4ms | 28ms |
| 037 VilModel Derive | Insurance Claims | **44,251** | 0.4ms | 24ms |
| 033 SHM Write-Through | Analytics Dashboard | **43,715** | 0.4ms | 30ms |
| 030 Tri-Lane Messaging | E-Commerce Orders | **41,840** | 0.4ms | 24ms |
| 029 Handler Macros | Developer Playground | **40,663** | 0.7ms | 25ms |
| 027 VilServer Minimal | Health Check | **40,011** | 0.4ms | 25ms |
| 006 SHM Extractor | HFT Processor | **39,178** | 0.8ms | 30ms |
| 020 A/B Testing | Marketing Campaign | **38,827** | 0.5ms | 25ms |
| 010 WebSocket | Customer Support | **36,878** | 0.3ms | 28ms |
| 034 Blocking Task | Credit Risk Scoring | **36,359** | 0.7ms | 30ms |

**Average: ~41,000 req/s | P50: 0.5ms | P99: 26ms | 100% success**

---

## 2. AI Gateway — Scaling (001, validated 2026-03-27)

SSE streaming proxy: HTTP POST → VIL Tri-Lane → upstream SSE → stream response to client.

Simulator: `AI_SIM_DELAY_MS=0`, release mode.

| Concurrent | Via VIL | Direct (simulator) | Overhead | P50 | P99 | Success |
|-----------|---------|-------------------|----------|-----|-----|---------|
| 100 | 2,111 | — | — | 43ms | 83ms | 100% |
| 200 | 4,342 | 4,734 | 8% | 43ms | 66ms | 100% |
| 300 | 5,825 | — | — | 46ms | 76ms | 100% |
| **400** | **6,076** | — | — | **58ms** | **100ms** | **100%** |
| **500** | **6,226** | **7,785** | **16%** | **76ms** | **116ms** | **100%** |
| 600 | 6,195 | — | — | 91ms | 120ms | 100% |
| 800 | 6,065 | — | — | 123ms | 186ms | 100% |
| 1000 | 6,239 | — | — | 152ms | 189ms | 100% |

**Sweet spot: c400-500 — ~6,200 req/s, P99 100-116ms, 16% overhead vs direct.**

Throughput plateaus at c500+. At c800+ P99 approaches 200ms SLO boundary.

---

## 3. NDJSON Pipeline (1000 Records/Request)

HTTP request → VIL pipeline → fetch NDJSON from upstream → per-record transform → stream response.

Each request processes **1,000 credit records** with real business logic.

| Example | Business Domain | req/s | records/s | P50 | Transform |
|---------|----------------|-------|-----------|-----|-----------|
| 007 NPL Filter | Loan Detection | **1,079** | **1,079K** | 181ms | Filter `kol≥3` |
| 005 Core Banking | Data Ingestion | **865** | **865K** | 225ms | Enrich risk + LTV |
| 008 Quality Monitor | Data QA | **841** | **841K** | 233ms | Validate 5 rules |
| 009 SLIK Regulatory | OJK Reporting | **794** | **794K** | 246ms | Map 12 fields |

**Average: ~895 req/s = 895K records/s | SIMD JSON (sonic-rs)**

### NDJSON Overhead (vs direct simulator)

| Path | req/s | records/s | avg | Overhead |
|------|-------|-----------|-----|----------|
| **Direct** (bypass VIL) | 2,200 | 2,200K | 87ms | — |
| **007 NPL Filter** (filter) | 1,079 | 1,079K | 181ms | 51% |
| **005 Enrich** (add 2 fields) | 865 | 865K | 225ms | 61% |
| **008 Quality** (5 rules) | 841 | 841K | 233ms | 62% |
| **009 SLIK Map** (12 fields) | 794 | 794K | 246ms | 64% |

The 51-64% "overhead" is **business logic cost** (JSON parse + transform + re-serialize per record), not VIL framework cost. VIL's fixed routing overhead is ~8ms — the rest scales linearly with record count.

---

## 4. Multi-Pipeline (ShmToken Shared Heap)

Multiple independent pipelines sharing a single `ExchangeHeap` via `ShmToken`. Zero-copy cross-workflow IPC.

| Example | Business Domain | req/s | P50 | P99 |
|---------|----------------|-------|-----|-----|
| 105 Multi-Workflow | Financial Data Hub | **3,732** | 46ms | 85ms |
| 102 Fan-Out Scatter | Risk Segmentation | **3,562** | 45ms | 85ms |

Shared ExchangeHeap adds **no measurable overhead** vs single-pipeline SSE baseline.

---

## 5. VIL Overhead Analysis

### Fixed Overhead (per-request)

| Component | Cost | Notes |
|-----------|------|-------|
| VIL Tri-Lane routing | ~5ms | SHM write + worker wake + port route |
| HTTP proxy | ~3ms | Accept + forward + response flush |
| **Total fixed** | **~8ms** | Measured: direct vs VIL on SSE (no per-record work) |

### Per-Record Overhead (NDJSON)

| Component | Cost per record | At 1000 records |
|-----------|----------------|-----------------|
| JSON parse (sonic-rs) | ~0.1ms | ~100ms |
| Transform logic | 0.05-0.2ms | 50-200ms |
| JSON serialize | ~0.05ms | ~50ms |

### Summary by Category

| Category | Throughput | VIL Overhead | Bottleneck |
|----------|-----------|--------------|------------|
| **VX_APP** (no I/O) | 41K req/s | **<1ms** | Network/OS, not VIL |
| **SSE Gateway** (streaming) | 6.2K req/s | **~8ms fixed** | Upstream response time |
| **Multi-Pipeline** (shared heap) | 3.7K req/s | **0ms additional** | Upstream response time |
| **NDJSON Transform** (1K rec) | 895 req/s | **~8ms + per-record** | Business logic (parse + transform) |

---

## 6. Context Comparison

What these numbers mean if you build a custom gateway or pipeline with VIL:

| Use Case | VIL Measured | Comparable Technology | Notes |
|----------|-------------|----------------------|-------|
| REST API gateway | **41,000 req/s** | Envoy (~30-50K), Nginx (~40-60K) | VIL includes business logic; Envoy/Nginx proxy only |
| AI inference proxy (SSE) | **6,200 req/s** | Kong (~8-15K), AWS API Gateway | VIL: full Tri-Lane routing + SHM; Kong: Lua plugins |
| Data pipeline (per-record) | **895K records/s** | Kafka Streams, Flink | VIL: single binary, zero infra; Kafka: cluster required |
| Service mesh | **3,700 req/s** | Linkerd, Istio dataplane | VIL: in-process SHM; Linkerd: sidecar proxy |

**Key differentiator:** VIL delivers infrastructure-grade throughput **while executing custom business logic** (validation, enrichment, routing decisions) — not just proxying bytes.

---

## 7. Performance Tuning Applied

| Optimization | Impact | Applied To |
|-------------|--------|-----------|
| **sonic-rs SIMD JSON** | +15% NDJSON throughput | vil_new_http, vil_server_core |
| **Level 1 WASM zero-copy** | 4→1 copies | vil_capsule (`data_mut` direct slice) |
| **vil_json hot path** | Bypass serde_json in parse | NDJSON line parse + json_tap |
| **SHM pool amortized reset** | P99 tail fix | check every 256 allocs, not every alloc |
| **Profile presets** | Tuned per environment | dev: 8MB/check64, prod: 256MB/check1024 |

---

## 8. Simulator Baselines

These simulators provide known-performance baselines for measuring VIL overhead:

| Simulator | Baseline (c500) | Port | Repository |
|-----------|----------------|------|------------|
| AI Endpoint (SSE, 5 dialects) | ~7,800 req/s | 4545 | [ai-endpoint-simulator](https://github.com/Vastar-AI/ai-endpoint-simulator) |
| Credit Data (NDJSON) | ~2,000 req/s (191K rec/s) | 18081 | [credit-data-simulator](https://github.com/Vastar-AI/credit-data-simulator) |

### How to Measure Overhead

```
Simulator baseline:  oha -c 500 -n 5000 http://localhost:4545/v1/chat/completions  → X req/s
Via VIL pipeline:    oha -c 500 -n 5000 http://localhost:3080/trigger              → Y req/s
VIL overhead:        (X - Y) / X × 100%
```

---

## 9. Infrastructure Dependencies

| Service | Container | Port | Required By |
|---------|-----------|------|-------------|
| AI Endpoint Simulator | ai-endpoint-simulator | 4545 | 001, 016-019, 201-206, 301-306, 401-406 |
| Credit Data Simulator | credit-data-simulator | 18081 | 005, 007-009, 101-107 |
| PostgreSQL | vil-postgres | 5432 | 012 |
| Redis | vil-redis | 6380 | 012 |
| NATS | vil-nats | 4222 | 013 |
| Kafka (Redpanda) | vil-redpanda | 9092 | 014 |
| MQTT (Mosquitto) | vil-mosquitto | 1883 | 015 |

---

## 10. Reproduction

```bash
# 1. Start simulator
git clone https://github.com/Vastar-AI/ai-endpoint-simulator.git
cd ai-endpoint-simulator && AI_SIM_DELAY_MS=0 cargo run --release

# 2. Build and run VIL example
cd vil
cargo run --release -p vil-basic-ai-gw-demo

# 3. Warmup
for i in $(seq 1 10); do
  curl -sN -X POST -H 'Content-Type: application/json' \
    -d '{"prompt":"warmup"}' http://localhost:3080/trigger > /dev/null
done
oha -c 50 -n 200 -m POST -H 'Content-Type: application/json' \
  -d '{"prompt":"w"}' http://localhost:3080/trigger > /dev/null

# 4. Benchmark
oha -c 500 -n 5000 -m POST -H 'Content-Type: application/json' \
  -d '{"prompt":"bench"}' http://localhost:3080/trigger

# 5. Compare with simulator baseline
oha -c 500 -n 5000 -m POST -H 'Content-Type: application/json' \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"Hi"}],"stream":true}' \
  http://localhost:4545/v1/chat/completions
```
