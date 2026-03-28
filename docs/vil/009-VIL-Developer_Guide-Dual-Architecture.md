# VIL Developer Guide — Part 9: Dual Architecture — VilApp vs ShmToken Pipeline

**Series:** VIL Developer Guide (9 of 9)
**Previous:** [Part 8 — Connectors & Semantic Types](./008-VIL-Developer_Guide-Connectors.md)
**Last updated:** 2026-03-28

---

## 1. Two Architectures, One Runtime

VIL provides two distinct execution architectures built on the same foundation (`VastarRuntimeWorld`, `ExchangeHeap`, Tri-Lane Protocol):

| | **VilApp** (Part 3) | **ShmToken Pipeline** (Part 4) |
|--|:---:|:---:|
| Entry point | `VilApp::new()` | `vil_workflow!` macro |
| Crate | `vil_server` | `vil_sdk` |
| HTTP layer | Axum (built-in) | `HttpSink` / `HttpSource` |
| Inter-stage transport | HTTP handler calls | SHM zero-copy descriptors |
| Observer | `.observer(true)` | `vil_observer::sidecar()` |
| Best for | HTTP APIs, proxies, gateways | Multi-stage ETL, data pipelines |

They are **not competing alternatives** — they solve different problems and can coexist in the same system.

---

## 2. VilApp — Process-Oriented HTTP Server

VilApp wraps Axum with VIL's process model. Each `ServiceProcess` is a named VIL Process with typed endpoints, and the Tri-Lane mesh routes signals between services.

```rust
use vil_server::prelude::*;

let api = ServiceProcess::new("api")
    .endpoint(Method::POST, "/trigger", post(handler));

VilApp::new("gateway")
    .port(8080)
    .observer(true)
    .service(api)
    .run()
    .await;
```

**Data flow:** Client → HTTP → Axum handler → business logic → HTTP response

### When to use VilApp

- REST API servers
- Single-hop HTTP proxies / API gateways
- WebSocket / SSE servers
- Services that need built-in health/ready/metrics endpoints
- Rapid prototyping (one crate, familiar Axum patterns)

### Strengths

- **Simpler mental model** — handlers are async functions, same as vanilla Axum
- **Built-in observer** — `.observer(true)` gives you a full dashboard
- **Lower memory** — 142 MB for a single proxy (no SHM overhead for simple workloads)
- **Fewer threads** — 17 (Tokio worker pool only)
- **ServiceProcess mesh** — inter-service routing via `VxMeshConfig` without HTTP hops

---

## 3. ShmToken Pipeline — Zero-Copy Data Pipeline

ShmToken pipeline uses `vil_workflow!` to wire typed nodes (`HttpSink`, `HttpSource`) with explicit Tri-Lane routing. Data transfers between nodes happen via SHM descriptors — no serialization, no copies.

```rust
use vil_sdk::prelude::*;

let world = Arc::new(VastarRuntimeWorld::new_shared()?);

let sink = configure_webhook_sink();    // HttpSinkBuilder
let source = configure_sse_source();    // HttpSourceBuilder

let (_ir, (sink_h, source_h)) = vil_workflow! {
    name: "Pipeline",
    instances: [sink, source],
    routes: [
        sink.trigger_out -> source.trigger_in (LoanWrite),
        source.response_data_out -> sink.response_data_in (LoanWrite),
        source.response_ctrl_out -> sink.response_ctrl_in (Copy),
    ]
};

// Observer sidecar on separate port
vil_observer::sidecar(3180).attach(&world).spawn();

let t1 = HttpSink::from_builder(sink).run_worker::<ShmToken>(world.clone(), sink_h);
let t2 = HttpSource::from_builder(source).run_worker::<ShmToken>(world.clone(), source_h);
t1.join().unwrap();
t2.join().unwrap();
```

**Data flow:** Client → HttpSink → SHM descriptor → Transform → SHM descriptor → HttpSource → upstream → SHM → HttpSink → Client

### When to use ShmToken Pipeline

- Multi-stage data processing (ETL, enrichment chains)
- High-throughput streaming (SSE, NDJSON)
- Fan-out / fan-in topologies
- Workloads where inter-stage serialization is the bottleneck
- Systems that need `LoanWrite` ownership transfer guarantees

### Strengths

- **Zero-copy between stages** — SHM descriptors, not serialized payloads
- **13% higher throughput** in multi-stage workloads (7,255 vs 6,399 req/s)
- **36% tighter P95** — 42ms vs 66ms (less jitter between stages)
- **Transform callbacks** — `.transform(|payload| ...)` runs in the data path without allocation
- **Explicit ownership** — `LoanWrite`, `Copy`, `ShareRead` transfer modes prevent data races at compile time

---

## 4. Head-to-Head Benchmark

All benchmarks: Intel i9-11900F, 16 threads, 32GB RAM, release build, `RUST_LOG=error`, upstream RAI simulator at :4545.

### Single Pipeline (HTTP proxy → upstream SSE)

Same business: receive webhook, forward to upstream LLM, stream response back.

| Metric | VilApp | ShmToken | Winner |
|--------|:------:|:--------:|:------:|
| Req/s | **4,530** | 3,637 | VilApp +25% |
| P50 | 44ms | 48ms | VilApp |
| P95 | 62ms | 73ms | VilApp |
| P99 | 73ms | 84ms | VilApp |
| Memory | 124 MB | 121 MB | ~same |

**For single-hop proxy, VilApp wins** — no SHM pipeline overhead needed.

### Multi-Pipeline (3-stage: webhook → transform → upstream)

Same business with an added transform stage that enriches each payload.

| Metric | VilApp | ShmToken | Winner |
|--------|:------:|:--------:|:------:|
| Req/s | 6,399 | **7,255** | ShmToken +13% |
| P50 | ~47ms | ~41ms | ShmToken |
| P95 | 66ms | **42ms** | ShmToken -36% |
| P99 | 75ms | **43ms** | ShmToken -43% |
| P99.9 | 94ms | 63ms | ShmToken -33% |
| Memory | 124 MB | 121 MB | ~same |

**For multi-stage, ShmToken wins** — zero-copy SHM eliminates serialization at each hop.

### Why ShmToken P95 is So Much Tighter

In VilApp, each "stage" is a function call within a single async handler. Tokio's task scheduler can preempt between stages, adding jitter. The P95-P99 spread is 66→75ms = 9ms of scheduling noise.

In ShmToken pipeline, each stage runs on its own dedicated thread. Data flows via SHM descriptors with `LoanWrite` — no task scheduling, no serialization. The P95-P99 spread is 42→43ms = **0.6ms** of jitter. Nearly zero.

---

## 5. Observer in Both Architectures

### VilApp Observer

```rust
VilApp::new("app")
    .observer(true)  // embedded dashboard at /_vil/dashboard/
```

Dashboard runs on the same port. Tracks:
- Per-route metrics (requests, req/s, avg, P95, P99, P99.9)
- Upstream calls (per-URL tracking via SseCollect)
- Req/s live chart (Grafana-style smooth curve)

**Overhead: 0%** — lock-free atomic counters, conditional middleware attachment.

### SDK Pipeline Observer (Sidecar)

```rust
vil_observer::sidecar(3180).attach(&world).spawn();
```

Dashboard runs on a separate port. Tracks:
- HTTP inbound requests (via global `AtomicU64` in HttpSink)
- Session latency with P95/P99/P99.9 (40-bucket histogram)
- Pipeline counters (SHM publishes, receives, drops, crashes)
- System metrics (PID, CPU, memory, threads)

---

## 6. Decision Matrix

| Question | → VilApp | → ShmToken |
|----------|:--------:|:----------:|
| Building a REST API? | **Yes** | |
| Building a multi-stage ETL? | | **Yes** |
| Single HTTP proxy? | **Yes** | |
| Need P95 < 50ms under load? | | **Yes** |
| Familiar with Axum? | **Yes** | |
| Need fan-out/fan-in topology? | | **Yes** |
| Need observer dashboard? | **Yes** (built-in) | **Yes** (sidecar) |
| Need `.transform()` callbacks? | | **Yes** |
| Want minimal boilerplate? | **Yes** | |
| Need explicit ownership transfer? | | **Yes** |

### Hybrid Pattern

For systems that need both an API and a data pipeline:

```
                    ┌─── VilApp (:8080) ───────────────────┐
                    │  ServiceProcess "api"                 │
  Client ──HTTP──▶  │    POST /ingest → publish to SHM     │
                    │  ServiceProcess "dashboard"           │
                    │    GET /status → read from SHM        │
                    └──────────┬────────────────────────────┘
                               │ SHM ExchangeHeap
                    ┌──────────▼────────────────────────────┐
                    │  vil_workflow! (ShmToken pipeline)     │
                    │    Stage 1: Validate                  │
                    │    Stage 2: Enrich                     │
                    │    Stage 3: Store                      │
                    └───────────────────────────────────────┘
```

VilApp handles the HTTP boundary. ShmToken handles the data pipeline. Both share the same `VastarRuntimeWorld` and `ExchangeHeap` — zero-copy from HTTP ingress to pipeline output.

---

## 7. Common Misconceptions

### "ShmToken is always faster"

No. For single-hop proxy, VilApp is 25% faster. SHM pipeline has per-stage thread dispatch overhead that only pays off when you have multiple stages transferring data between them.

### "VilApp can't do zero-copy"

VilApp uses `ShmSlice` for request bodies — the body is allocated in `ExchangeHeap` and handlers access it via zero-copy slice. The zero-copy is at the HTTP boundary, not between stages.

### "They can't coexist"

They share the same `VastarRuntimeWorld`. A single binary can run a VilApp for its HTTP API and a ShmToken pipeline for its data processing, sharing the same SHM region.

### "Observer only works with VilApp"

Observer works in both modes. VilApp uses `.observer(true)` (embedded). SDK pipeline uses `vil_observer::sidecar(port).attach(&world).spawn()` (separate port). Both provide the same dashboard with P95/P99/P99.9, live charts, and system metrics.

---

## 8. Migration Path

### VilApp → ShmToken

When your VilApp handler grows to 3+ stages with heavy data transformation, consider extracting the pipeline into a `vil_workflow!`:

1. Keep VilApp for HTTP ingress
2. Move transform logic to `HttpSourceBuilder::transform()`
3. Wire via `vil_workflow!` with `LoanWrite` routes
4. Benefit: 13% throughput gain, 36% tighter P95

### ShmToken → VilApp

When your pipeline needs more HTTP endpoints (health, admin, WebSocket):

1. Wrap the pipeline trigger in a `ServiceProcess` endpoint
2. Add other services (health, dashboard, admin) via VilApp
3. Both share `VastarRuntimeWorld`

---

> **Part 3** covers VilApp in depth. **Part 4** covers ShmToken pipeline in depth. This guide explains when and why to choose each.

> **Examples:** `001` (ShmToken proxy), `001b` (VilApp proxy), `101b` (ShmToken multi-pipeline), `101c` (VilApp multi-pipeline)
