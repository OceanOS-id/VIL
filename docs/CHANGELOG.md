# Changelog

All notable changes to VIL are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

### AI Plugin Infrastructure — VIL Process-Oriented Refactor (2026-03-21)

51 AI crates reimplemented with VIL pattern (Phase 6 complete 2026-03-26 — all 51/51 VIL Way):
- **5 VIL layers per crate**: semantic types (Tier B), SSE pipeline builders, VilPlugin + ServiceProcess, REST handlers, core logic
- **SseCollect**: built-in async SSE client with dialect system (OpenAI, Anthropic, Ollama, Cohere, Gemini)
- **ShmToken**: zero-copy SHM transport (2% overhead, 4,400 req/s)
- **15 new examples** (026-040): LLM, RAG, Agent plugin usage with dialect demos
- **Transport classification**: VilApp+SseCollect for single-proxy, vil_sdk+ShmToken for multi-stage
- **Upstream auth**: bearer_token, anthropic_key, api_key_param per dialect
- **Prelude enhanced**: Extension, EndpointSpec, Arc, SseCollect, SseDialect, reqwest

Crate tiers: 3 Official Plugins + 5 Tier 0A + 15 Tier 0B + 8 Tier 1 + 10 Tier 2 + 12 Tier 3
Tests: 1,425 passing | Examples: 40 | Semantic types: 164

---

## [Unreleased] — Observer Dashboard Integration

### Observer Dashboard (2026-03-28)

**Core Integration:**
- **`VilApp::observer(true)`** — enable embedded dashboard at `/_vil/dashboard/` with one builder call
- **`VilServer::observer(true)`** — lower-level builder support
- **Sidecar mode** — `vil_observer::sidecar(port).attach(&world).spawn()` for SDK pipelines (1 line)
- **10 JSON API endpoints** — topology, metrics, health, routes, upstreams, shm, logs/recent, system, config, pipeline
- **True zero overhead** — when observer OFF, metrics middleware **not attached** (not disabled, not there)

**Metrics & Histograms:**
- **40-bucket latency histogram** — 10us→5s, lock-free `AtomicU64`, P95/P99/P99.9 per route
- **Microsecond precision** — `duration_sum_us` (not ms), CAS atomic min/max tracking
- **Upstream tracking** — `UpstreamRegistry` with global singleton, auto-records `SseCollect` outbound calls
- **Inbound tracking** — `HttpSink` global counters for SDK pipeline mode (request count + session latency)
- **`/_vil/` routes excluded** — dashboard polling doesn't pollute business metrics

**Dashboard UI:**
- **Throughput** — Total Requests, Req/s (live smoothed + avg), Success Rate, Slowest/In-Flight, Memory RSS
- **Response Time Distribution** — Fastest, Average, P95, P99, P99.9
- **Req/s Live Chart** — Canvas, monotone cubic spline (Grafana-style), configurable refresh (1s-10s, default 3s)
- **Upstreams Table** — per-URL: requests, req/s, in-flight, avg, P95, P99, P99.9, error rate, status
- **Routes Table** — per-route: method, path, class, requests, req/s, avg, P95, P99, P99.9, error rate
- **Pipeline Counters** (sidecar) — publishes, receives, drops, crashes, orphans, hops, failovers
- **System/Config** — PID, CPU, threads, FDs, memory RSS, Rust version, VIL version, health

**SDK & Codegen:**
- **YAML manifest** — `observer: true` field, all 12 templates
- **Codegen** — auto-generates `.observer(true)` from YAML
- **Semantic events** — `ObserverMetricsSnapshot`, `ObserverDashboardAccess`, `ObserverErrorAlert`

**Benchmark Results (release, i9-11900F, 16 threads):**
- Observer ON vs OFF: **0% overhead** (4,646 vs 4,611 req/s)
- VilApp single proxy: **6,608 req/s**, 142 MB, P95=61ms
- ShmToken multi-pipeline: **7,255 req/s** (+13%), P95=42ms (-36% vs VilApp)
- Concurrency sweet spot: c=300 (6,561 req/s, P99<100ms)

**Examples & Tests:**
- `039-basic-observer-dashboard` — VilApp observer example
- `001b-vilapp-ai-gw-benchmark` — observer ON/OFF toggle benchmark
- `101b-multi-pipeline-benchmark` — ShmToken 3-stage pipeline
- `101c-vilapp-multi-pipeline-benchmark` — VilApp 3-stage pipeline
- 2 codegen tests, 7 SDK YAML tests, 2 unit tests, 15-example regression (zero failures)

**Version bumps:** `vil_observer` 0.1.0→0.1.1, `vil_server_core` 0.1.0→0.1.1

---

## [Unreleased] — Configuration Architecture & Performance Tuning

### Configuration Architecture (2026-03-26)
- **Profile system**: `dev` / `staging` / `prod` presets with tuned defaults per environment
- **FullServerConfig**: expanded with `pipeline`, `database` (postgres, redis), `mq` (nats, kafka, mqtt) sections
- **3-layer precedence**: Code Default → YAML → Profile → ENV (`VIL_*` env vars)
- **SHM P99 tuning**: amortized reset check (every N allocs, not every alloc), `ShmPoolConfig` with `check_interval`
- **VilApp::profile("prod")**: set heap_size from profile preset in builder chain
- **30+ env var overrides**: `VIL_DATABASE_URL`, `VIL_REDIS_URL`, `VIL_NATS_URL`, `VIL_KAFKA_BROKERS`, `VIL_SHM_*`, `VIL_PIPELINE_*`
- **Reference YAML**: `vil-server.reference.yaml` — complete config reference with all options documented
- **Profile tuning summary**:
  - dev: 8MB SHM, debug logging, 5 DB connections, admin enabled
  - staging: 64MB SHM, JSON logging, rate limits on, trace sampling 1:10
  - prod: 256MB SHM, check_interval=1024, 50 DB connections, admin disabled, HSTS+compression on

### Performance Tuning (2026-03-26)
- **SIMD JSON** (sonic-rs): +15% NDJSON throughput via `vil_json` simd feature
- **WASM Level 1 Zero-Copy**: `memory.data_mut()` direct slice access (1 copy, not 4)
- **SHM Pool amortized reset**: P99 tail latency fix — check every 256 allocs instead of every alloc
- **Benchmark results**: VX_APP ~41K req/s (P50 0.5ms), NDJSON ~895K records/s, VIL overhead ~8ms

---

## [Unreleased] — Community Edition Hardening

### Added
- **vil_sidecar** crate: protocol (48B VASI descriptor), UDS transport, SHM bridge, registry, lifecycle, metrics, dispatcher, failover
- **WASM FaaS**: WasmPool (instance pooling), WasmFaaSConfig, WasmFaaSRegistry in vil_capsule
- **ExecClass**: `SidecarProcess` and `WasmFaaS` variants for hybrid execution
- **VilApp::sidecar()**: register sidecar configs in process topology
- **CircuitBreaker**: threshold-based circuit breaker with cooldown for sidecar failover
- **Failover dispatcher**: primary → backup sidecar → WASM fallback
- **Admin endpoints**: 6 REST endpoints for sidecar management (/admin/sidecars/*)
- **CLI**: `vil sidecar` subcommand (list, health, attach, drain, metrics)
- **Sidecar SDK**: Python (`vil_sidecar` package) and Go (`vil_sidecar` module)
- **Examples**: 020 (WASM FaaS), 021 (Python sidecar), 022 (hybrid pipeline)
- **Integration tests**: hybrid example tests in test_examples.sh

### Community Edition Hardening (2026-03-20)
- **TCP Tri-Lane Transport**: Cross-host communication via length-prefixed binary protocol, TcpTriLaneRouter with auto SHM/TCP selection, persistent connections with reconnect
- **Semantic Macro Fix**: `pub use vil_sdk;` in vil_server — `#[vil_state]` now works in server context
- **Real WASM Execution**: 3 WASM modules (pricing, validation, transform), CapsuleHost.precompile() + call_i32() + call_with_memory()
- **Sidecar Connection Pool**: ConnectionPool with round-robin, backpressure (max_in_flight), PooledConnection with auto-decrement
- **Auto-Reconnect**: ReconnectPolicy with exponential backoff, configurable jitter, max retries
- **Python Sidecar SDK**: `vil_sidecar.py` with `@sidecar.method()` decorator
- **Go Sidecar SDK**: `VilSidecar` with `Method()` registration and `Run()`
- **vil_lsp**: Language Server Protocol — diagnostics, completions, hover for VIL macros in Rust files
- **VS Code Extension**: `editors/vscode/vil/` — extension skeleton for vil-lsp integration
- **vil_observer**: Embedded observer dashboard at `/_vil/dashboard/` with MetricsCollector, dark-theme SPA, 5s auto-refresh

---

## [Previous Unreleased]

*Initial public release preparation.*

---

## Versioning Policy

- **MAJOR** (X.0.0): Breaking changes to public API or core semantics
- **MINOR** (0.X.0): New features, backward compatible
- **PATCH** (0.0.X): Bug fixes only

---

## Support

- **Current Status**: Active Development
- **MSRV**: Rust 1.70+
- **License**: Apache-2.0

For issues or questions, visit:
- [GitHub Issues](https://github.com/OceanOS-id/VIL/issues)
- [GitHub Discussions](https://github.com/OceanOS-id/VIL/discussions)
