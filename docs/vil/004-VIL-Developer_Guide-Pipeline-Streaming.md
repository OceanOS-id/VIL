# VIL Developer Guide — Part 4: Pipeline & HTTP Streaming

**Series:** VIL Developer Guide (4 of 9)
**Previous:** [Part 3 — Server Framework](./003-VIL-Developer_Guide-Server-Framework.md)
**Next:** [Part 5 — Infrastructure & Plugins](./005-VIL-Developer_Guide-Infrastructure.md)
**Last updated:** 2026-03-26

---

## 1. Building Pipelines: `vil_workflow!`

VIL pipelines connect nodes (sources, sinks, processors) via Tri-Lane routes. You have three equally performant styles:

### 1.1 Style A: Macro-Heavy (Declarative)

Highly concise, ideal for standard pipelines:

```rust
let (_ir, handles) = vil_workflow! {
    name: "StandardPipeline",
    instances: [ sink, source ],
    routes: [
        sink.out -> source.in (LoanWrite),
        source.data -> sink.in (LoanWrite),
    ]
};
```

### 1.2 Style B: Decomposed Builder (Modular)

Suited for large systems. Node configuration is separated into independent functions:

```rust
let sink_builder   = configure_sink();
let source_builder = configure_source();

let (_ir, handles) = vil_workflow! {
    name: "ModularPipeline",
    instances: [ sink_builder, source_builder ],
    routes: [ /* wiring identical to macro style */ ]
};
```

### 1.3 YAML Topology Projection (`vil_topo`)

For very large distributed systems, separate infrastructure declarations into YAML files:

```yaml
# topology.yaml
name: ClusterGateway
hosts:
  edge_node: "10.0.0.1:9000"
  core_node: "10.0.0.2:9000"
instances:
  - name: ingress @ edge_node
    type: HttpSink
  - name: logic @ core_node
    type: CreditProcessor
```

Run `vil_topo topology.yaml` to generate Rust boilerplate. This separates DevOps concerns (Topology) from Developer concerns (Logic).

---

## 2. HTTP Streaming with `vil_new_http`

`vil_new_http` is VIL's **sole HTTP streaming crate** for building SSE and NDJSON pipelines. It provides two core components:

- **`HttpSinkBuilder`** — Accepts incoming HTTP requests (webhook trigger), extracts the request body, and forwards it upstream.
- **`HttpSourceBuilder`** — Connects to an upstream HTTP endpoint, streams SSE or NDJSON data back to the sink.

> **Note:** `vil_http` has been archived. All HTTP streaming pipelines use `vil_new_http` exclusively.

### 2.1 Architecture

```
Client ──HTTP POST──> HttpSink (:3080/trigger)
                          │
                     [Trigger Lane]
                          │
                          v
                      HttpSource ──GET/POST──> Upstream SSE/NDJSON
                          │
                     [Data Lane - streaming]
                          │
                          v
                      HttpSink ──SSE/chunks──> Client
```

### 2.2 SSE Source (Server-Sent Events)

```rust
use vil_sdk::http::{HttpSourceBuilder, HttpFormat, SseSourceDialect};

let source = HttpSourceBuilder::new()
    .url("http://localhost:18081/api/v1/credits/stream")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::Standard)  // W3C standard SSE
    .build();
```

### 2.3 NDJSON Source (Newline-Delimited JSON)

```rust
use vil_sdk::http::{HttpSourceBuilder, HttpFormat};

let source = HttpSourceBuilder::new()
    .url("http://localhost:18081/api/v1/credits/ndjson")
    .format(HttpFormat::NDJSON)
    .build();
```

The NDJSON runtime parses line-by-line using a `BytesMut` buffer. Each complete line (terminated by `\n`) is emitted as a separate data frame through the Data Lane.

### 2.4 HTTP Sink (Webhook Trigger)

```rust
use vil_sdk::http::HttpSinkBuilder;

let sink = HttpSinkBuilder::new()
    .port(3080)
    .path("/trigger")
    .build();
```

The default HTTP method for `HttpSourceBuilder` is **GET** (set at `vil_new_http/source.rs:80`). POST is used when calling AI inference endpoints that require a request body.

### 2.5 SSE Dialect System

VIL supports 7 built-in SSE dialects for different upstream providers:

| # | Dialect | Constructor | Done Signal | `json_tap` Path |
|---|---------|------------|-------------|-----------------|
| 1 | **OpenAI** | `SseSourceDialect::OpenAi` | `data: [DONE]` | `choices[0].delta.content` |
| 2 | **Anthropic** | `SseSourceDialect::Anthropic` | `event: message_stop` | `delta.text` |
| 3 | **Ollama** | `SseSourceDialect::Ollama` | `"done": true` | `message.content` |
| 4 | **Cohere** | `SseSourceDialect::Cohere` | `"is_finished": true` | `text` |
| 5 | **Gemini** | `SseSourceDialect::Gemini` | `"done": true` | `candidates[0].content.parts[0].text` |
| 6 | **Standard** | `SseSourceDialect::Standard` | TCP EOF | (raw text) |
| 7 | **Custom** | `SseSourceDialect::Custom(...)` | configurable | configurable |

**Standard dialect** is used for non-AI SSE endpoints (e.g., Core Banking Simulator, generic event streams). It relies on TCP connection close (EOF) to signal completion.

### 2.6 `json_tap` — Content Extraction

The `json_tap` feature extracts a nested JSON field from each SSE event before forwarding:

```rust
// AI use case: extract content from OpenAI-format SSE
let source = HttpSourceBuilder::new()
    .url("http://localhost:4545/v1/chat/completions")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::OpenAi)
    .json_tap("choices[0].delta.content")
    .build();

// Business use case: no json_tap needed — forward entire event
let source = HttpSourceBuilder::new()
    .url("http://localhost:18081/api/v1/credits/stream")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::Standard)
    .build();
```

### 2.7 `FromStreamData` Trait

For NDJSON and custom SSE processing, implement `FromStreamData`:

```rust
pub trait FromStreamData {
    fn from_ndjson_line(line: &[u8]) -> Option<Self> where Self: Sized;
    fn from_ndjson_line_shm(line: &[u8], heap: &ExchangeHeap) -> Option<Self> where Self: Sized;
}
```

---

## 3. Layered Public API

VIL provides three convenience API layers built on top of the core `vil_workflow!` macro:

### 3.1 Layer 1: Gateway (5 lines)

The fastest way to get a pipeline running — ideal for simple HTTP proxy gateways:

```rust
use vil_sdk::http_gateway;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    http_gateway()
        .listen(3080)
        .upstream("http://localhost:18081/api/v1/credits/stream")
        .sse(true)
        .run()?;
    Ok(())
}
```

### 3.2 Layer 2: Pipeline Builder (20 lines)

For custom topologies with multiple nodes, explicit route modes, and SSE/JSON tap:

```rust
use vil_sdk::{Pipeline, RouteMode};
use vil_sdk::http::{HttpSinkBuilder, HttpSourceBuilder, HttpFormat, SseSourceDialect};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pipeline = Pipeline::new("credit-gateway");

    let sink = pipeline.http_sink()
        .port(3080).path("/trigger")
        .build();

    let source = pipeline.http_source()
        .url("http://localhost:18081/api/v1/credits/stream")
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::Standard)
        .build();

    pipeline.route(sink, source, RouteMode::LoanWrite);
    pipeline.run()?;
    Ok(())
}
```

### 3.3 Layer 3: Full Macro DSL

See Section 1 (`vil_workflow!`) for the full declarative macro approach, which gives complete control over topology wiring.

---

## 4. Business-Domain SSE Examples

Examples 004, 006, 007, 008 use the **Core Banking Simulator** (port 18081) for fintech-domain SSE streaming:

### 4.1 Core Banking Simulator Endpoints

| Endpoint | Format | Description |
|----------|--------|-------------|
| `GET /api/v1/credits/stream` | SSE | Batched credit records (`event: records` + `event: complete`) |
| `GET /api/v1/credits/ndjson` | NDJSON | One JSON credit record per line |

**Query parameters:** `count`, `dirty_ratio`, `seed`, `batch_size`, `delay_ms`

**Credit record fields:** `id`, `nik`, `nama_lengkap`, `kolektabilitas` (1-5), `jumlah_kredit`, `saldo_outstanding`, `_has_error`, `_error_type`

### 4.2 Example 004: Multi-Service Mesh with Core Banking SSE

Three ServiceProcess instances communicating via Tri-Lane mesh, with upstream Core Banking SSE as data source.

### 4.3 Example 006: Credit NPL Stream Filter

Filters credit records with `kolektabilitas >= 3` (Non-Performing Loans). Runs on port 3081 at `/filter-npl`.

### 4.4 Example 007: Credit Data Quality Monitor

Monitors credit data quality by detecting `_has_error: true` records. Runs on port 3082 at `/quality-check`.

### 4.5 Example 008: Credit Regulatory Stream Pipeline (SLIK/OJK)

Transforms credit data into regulatory report format for SLIK (Sistem Layanan Informasi Keuangan) / OJK compliance. Runs on port 3083 at `/regulatory-stream`.

---

## 5. AI SSE Examples

Examples 001, 015, 017-018, 023-040 use the **AI SSE Simulator** (port 4545) for LLM/RAG/Agent streaming:

```rust
// AI gateway pattern — OpenAI dialect with json_tap
let source = pipeline.http_source()
    .url("http://localhost:4545/v1/chat/completions")
    .format(HttpFormat::SSE)
    .dialect(SseSourceDialect::OpenAi)
    .json_tap("choices[0].delta.content")
    .build();
```

These examples are intentionally AI-centric — they demonstrate LLM proxy, RAG pipeline, multi-model routing, A/B testing, and plugin composition patterns.

---

## 6. YAML Pipeline Definitions

Pipelines can be defined declaratively in YAML for deployment without recompilation:

```yaml
name: credit-gateway
version: "1.0"

nodes:
  webhook:
    type: http-sink
    port: 3080
    path: /trigger

  banking:
    type: http-source
    url: http://localhost:18081/api/v1/credits/stream
    format: sse

routes:
  - from: webhook.trigger_out
    to: banking.trigger_in
    mode: LoanWrite
  - from: banking.data_out
    to: webhook.data_in
    mode: LoanWrite
  - from: banking.ctrl_out
    to: webhook.ctrl_in
    mode: Copy

observability:
  prometheus:
    port: 9090
```

Run with: `vil run --file pipeline.vil.yaml`
Validate with: `vil validate pipeline.vil.yaml`

---

## 7. `SseCollect` — Built-in SSE Client (Server-Side)

For server-side AI plugin integration, `SseCollect` provides a built-in async SSE client with dialect support:

```rust
use vil_server::prelude::*;

// Zero-setup (built-in client, OpenAI dialect default)
let content = SseCollect::post_to("http://localhost:4545/v1/chat/completions")
    .body(json!({"model": "gpt-4", "messages": [...], "stream": true}))
    .collect_text().await?;

// With explicit dialect
let content = SseCollect::post_to(url)
    .dialect(SseDialect::anthropic())
    .body(body)
    .collect_text().await?;
```

`SseCollect` uses a global connection-pooled `reqwest::Client` (lazy-initialized, `tcp_nodelay`, 100 max idle per host).

**Note:** `SseCollect` is for server-side AI proxy patterns. For pipeline streaming, use `vil_new_http`'s `HttpSourceBuilder`.

---

## What's New (2026-03-26)

### `HttpSourceBuilder::transform()` — Inline Stream Processing

The new `.transform()` method on `HttpSourceBuilder` enables inline NDJSON/SSE processing without requiring a separate processor node. This is ideal for simple filter/map operations:

```rust
let source = HttpSourceBuilder::new()
    .url("http://localhost:18081/api/v1/credits/ndjson")
    .format(HttpFormat::NDJSON)
    .transform(|line: &[u8]| -> Option<Vec<u8>> {
        let record: CreditRecord = serde_json::from_slice(line).ok()?;
        if record.kolektabilitas >= 3 {
            Some(serde_json::to_vec(&record).unwrap())
        } else {
            None // filtered out
        }
    })
    .build();
```

The callback runs per-line for NDJSON or per-event for SSE. Returning `None` drops the record; returning `Some(bytes)` forwards the transformed payload.

### NDJSON Pipeline Examples (Tier 3)

Five new examples demonstrate NDJSON pipelines with real business transforms:

| Example | Port | Description |
|---------|------|-------------|
| 005-ndjson-basic | 3084 | Basic NDJSON passthrough pipeline |
| 007-ndjson-filter | 3085 | NPL filter with `.transform()` callback |
| 008-ndjson-enrich | 3086 | Field enrichment (risk scoring) |
| 009-ndjson-aggregate | 3087 | Running aggregation (portfolio stats) |
| 041-046 | 3088+ | Business-domain NDJSON with `#[vil_fault]` error handling |

### Multi-Pipeline Examples (Tier 4: 101-105)

Five advanced multi-pipeline examples demonstrate complex topologies:

| Example | Pattern | Description |
|---------|---------|-------------|
| 101 | **Fan-Out** | One source → N sinks (broadcast to multiple consumers) |
| 102 | **Fan-In** | N sources → one sink (merge from multiple upstreams) |
| 103 | **Diamond** | Fan-out → process → fan-in (parallel processing with merge) |
| 104 | **Multi-Workflow** | Independent pipelines sharing a common SHM region |
| 105 | **Conditional Routing** | Dynamic routing based on payload content |

Each uses `vil_workflow!` with multiple `HttpSourceBuilder` / `HttpSinkBuilder` instances wired through Tri-Lane routes.

### ShmToken vs GenericToken Usage Guide

When building pipelines, choose the appropriate token type:

```rust
// High-throughput streaming: use ShmToken (32 bytes, zero-alloc)
let (_ir, handles) = vil_workflow! {
    name: "StreamPipeline",
    token: ShmToken,  // <-- explicit token selection
    instances: [ sink, source ],
    routes: [ sink.out -> source.in (LoanWrite) ]
};

// Simple request-response or cross-host: use GenericToken (default)
let (_ir, handles) = vil_workflow! {
    name: "SimplePipeline",
    // token defaults to GenericToken
    instances: [ sink, source ],
    routes: [ sink.out -> source.in (LoanWrite) ]
};
```

Rule of thumb: if your pipeline handles >10K msg/s or uses fan-out, prefer `ShmToken`.

---

*Previous: [Part 3 — Server Framework](./003-VIL-Developer_Guide-Server-Framework.md)*
*Next: [Part 5 — Infrastructure & Plugins](./005-VIL-Developer_Guide-Infrastructure.md)*
