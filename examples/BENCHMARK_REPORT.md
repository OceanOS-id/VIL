# VIL Benchmark Report

**System:** Intel i9-11900F @ 2.50GHz (8C/16T), 32GB RAM, Ubuntu 22.04 (kernel 6.8.0)
**Rust:** 1.93.1 | **JSON:** sonic-rs SIMD | **Load tool:** oha
**Date:** 2026-03-27 | **Note:** Machine under normal dev workload (load avg ~1.3) during all benchmarks

## Baseline

| Example | Pattern | req/s | P50 | P99 | Notes |
|---------|---------|-------|-----|-----|-------|
| **001 AI Gateway** | SSE Pipeline | **3,473** | 47ms | 92ms | Upstream: AI Sim :4545 |

## VX_APP (HTTP Server — no external I/O)

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

**Average: ~41,000 req/s | P50: ~0.5ms | 100% success**

## SDK_PIPELINE (NDJSON Transform — 1000 records/request)

| Example | Business Domain | req/s | records/s | Transform |
|---------|----------------|-------|-----------|-----------|
| 007 NPL Filter | Loan Detection | **1,079** | **1,079K** | Filter kol≥3 |
| 005 Core Banking | Data Ingestion | **865** | **865K** | Enrich risk+LTV |
| 008 Quality Monitor | Data QA | **841** | **841K** | Validate 5 rules |
| 009 SLIK Regulatory | OJK Reporting | **794** | **794K** | Map 12 fields |

**Average: ~895 req/s = ~895K records/s with real business transforms (SIMD JSON)**

## Multi-Pipeline (ShmToken Shared Heap)

| Example | Business Domain | req/s | P50 | P99 |
|---------|----------------|-------|-----|-----|
| 105 Multi-Workflow | Financial Data Hub | **3,732** | 46ms | 85ms |
| 102 Fan-Out Scatter | Risk Segmentation | **3,562** | 45ms | 85ms |

## Bottleneck Analysis

| Category | Throughput | vs Baseline | Bottleneck |
|----------|-----------|-------------|------------|
| **VX_APP** (no I/O) | ~41K req/s | **12x** faster | None — pure VIL overhead <1ms |
| **SSE Pipeline** (AI sim) | ~3.5K req/s | **1x** | Upstream SSE response (~40ms) |
| **Multi-Pipeline** (shared heap) | ~3.6K req/s | **1x** | Upstream response time |
| **NDJSON Transform** (1K rec/req) | ~760 req/s | **0.22x** | Expected — streaming 1K records/req |
| **NDJSON raw throughput** | — | — | 760K records/s (actual data processing) |

## Upstream Direct vs Via VIL (Overhead Measurement)

### AI Simulator (SSE :4545)

| Path | req/s | avg | P50 | P99 | Overhead |
|------|-------|-----|-----|-----|----------|
| **Direct** (bypass VIL) | 4,429 | 41ms | 42ms | 69ms | — |
| **Via VIL** (001 pipeline) | 3,643 | 49ms | 46ms | 78ms | **17.7%** throughput, +8ms avg |

### Core Banking (NDJSON :18081, 1000 records/request)

| Path | req/s | avg | Transform | Overhead |
|------|-------|-----|-----------|----------|
| **Direct** (bypass VIL) | 2,200 | 87ms | none | — |
| **005 Enrich** (+risk+LTV) | 865 | 225ms | parse + add 2 fields/record | 61% |
| **007 NPL Filter** (kol≥3) | 1,079 | 181ms | parse + filter/record | 51% |
| **008 Quality** (5 rules) | 841 | 233ms | parse + validate/record | 62% |
| **009 SLIK Map** (12 fields) | 794 | 246ms | parse + remap 12 fields/record | 64% |

### Overhead Breakdown

| Component | Cost | Notes |
|-----------|------|-------|
| VIL Tri-Lane routing | ~5ms | SHM write + worker wake + port route |
| HTTP proxy | ~3ms | Accept + forward + response flush |
| JSON parse per record | ~0.1ms × 1000 | serde_json from NDJSON line |
| Transform per record | 0.05-0.2ms × 1000 | Business logic (filter/validate/map) |
| JSON serialize per record | ~0.05ms × 1000 | Output record serialization |

**VIL routing overhead: ~8ms fixed.** The rest is per-record processing that scales linearly with record count.

### Key Insights

1. **Pure VIL overhead: 8ms** — measured as direct vs VIL on SSE (no per-record work)
2. **VX_APP servers: <1ms overhead** — 41K req/s, bottleneck is network/OS not VIL
3. **SIMD JSON (sonic-rs): +15% NDJSON throughput** — enabled via `vil_json` simd feature
4. **NDJSON transform cost is linear** — 51-64% "overhead" is business logic (parse + transform), not VIL framework
5. **Blocking tasks scale well** — 034 (CPU-intensive) hits 36K req/s with spawn_blocking
6. **ShmToken multi-pipeline** matches SSE baseline — shared ExchangeHeap adds no overhead

### Performance Tuning Applied

| Optimization | Impact | Applied To |
|-------------|--------|-----------|
| **sonic-rs SIMD JSON** | +15% NDJSON throughput | vil_new_http, vil_server_core |
| **Level 1 WASM zero-copy** | 4→1 copies | vil_capsule (data_mut direct slice) |
| **vil_json hot path** | Bypass serde_json in parse | NDJSON line parse + json_tap |

### AI Endpoint Simulator Performance

**Multi-dialect SSE simulator** — supports OpenAI, Anthropic, Ollama, Cohere, Gemini.
Repository: https://github.com/Vastar-AI/ai-endpoint-simulator

**System:** Intel i9-11900F @ 2.50GHz (8C/16T), 32GB RAM, Ubuntu 22.04, Rust 1.93.1
**Build:** `--release` | **Delay:** 0ms | **Load avg:** ~1.3 (machine under normal dev workload)

#### Scaling (OpenAI dialect, 42 SSE tokens/response)

| Concurrent | Total Req | req/s | P50 | P99 | P99.9 | Success |
|-----------|-----------|-------|-----|-----|-------|---------|
| 500 | 5,000 | 5,652 | 81ms | 103ms | 125ms | 100% |
| 600 | 6,000 | 6,803 | 79ms | 110ms | 114ms | 100% |
| 700 | 7,000 | 7,486 | 87ms | 118ms | 138ms | 100% |
| 800 | 8,000 | 8,505 | 87ms | 115ms | 137ms | 100% |
| **1,000** | **10,000** | **9,944** | **91ms** | **141ms** | **160ms** | **100%** |
| 1,500 | 15,000 | 12,942 | 100ms | 208ms | 228ms | 100% |
| 2,000 | 20,000 | 8,715 | 218ms | 279ms | 303ms | 100% |
| 2,500 | 25,000 | 8,285 | 288ms | 354ms | 378ms | 100% |

**Sweet spot: c1000 — ~10K req/s, P99 141ms** (within 200ms SLO).
Peak throughput at c1500 (12.9K req/s) but P99 exceeds 200ms.
At c2000+ throughput drops due to connection contention.

#### Per-Dialect (c500, n5000)

| Dialect | req/s | P50 | P99 |
|---------|-------|-----|-----|
| OpenAI | 5,652 | 81ms | 103ms |
| Anthropic | 5,605 | 82ms | 111ms |
| Ollama | 5,730 | 80ms | 97ms |
| Cohere | 5,745 | 81ms | 94ms |
| Gemini | 5,567 | 82ms | 121ms |

All dialects perform equally — SSE format differences add no measurable overhead.
The simulator is NOT the bottleneck when benchmarking VIL pipeline examples.

### Infrastructure Dependencies

| Service | Container | Port | Required By |
|---------|-----------|------|-------------|
| AI Endpoint Simulator | ai-endpoint-simulator | 4545 | 001, 016-019, 201-206, 301-306, 401-406 |
| Core Banking Simulator | credit-data-simulator | 18081 | 005, 007-009, 101-107 |
| PostgreSQL | vil-postgres | 5432 | 012 |
| Redis | vil-redis | 6380 | 012 |
| NATS | vil-nats | 4222 | 013 |
| Kafka (Redpanda) | vil-redpanda | 9092 | 014 |
| MQTT (Mosquitto) | vil-mosquitto | 1883 | 015 |
