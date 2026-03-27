# VIL Developer Guide — Part 5: Infrastructure & Plugins

**Series:** VIL Developer Guide (5 of 7)
**Previous:** [Part 4 — Pipeline & HTTP Streaming](./004-VIL-Developer_Guide-Pipeline-Streaming.md)
**Next:** [Part 6 — CLI, Deployment & Best Practices](./006-VIL-Developer_Guide-CLI-Deployment.md)
**Last updated:** 2026-03-26

---

## 1. Resilience & Fault Model

Faults in VIL are not merely `Result<T, E>`. They are entities that flow through the **Control Lane** independently from the Data Lane.

### 1.1 Fault as Semantic Entity

Deriving `vil_fault` grants automatic Control Lane integration:

```rust
#[vil_fault]
pub enum PipelineFault {
    TransferFailed { port: PortId, reason: String },
    HostDown { host: HostId },
    LaneTimeout { lane: LaneId, elapsed_ms: u64 },
    CapacityExhausted { allocator: String },
}

// Auto-generated:
// - impl Into<ControlSignal> for PipelineFault
// - signal_error(fault) → send to Control Lane
// - control_abort(session_id) → terminate session
// - degrade(level) → operate in degraded mode
```

### 1.2 Declarative Failover

Failover is declared in the workflow — not imperative code:

```rust
vil_workflow! {
    name: "ResilientPipeline",
    routes: [
        source.out -> primary_sink.in (LoanWrite),
    ],
    failover: [
        primary_sink => backup_sink (
            on: HostDown,
            strategy: Immediate,
        ),
        source => retry(3, backoff: 100ms) (
            on: TransferFailed,
        ),
    ]
}
```

The compiler generates: monitoring hooks, reroute logic (using Phase 8 atomic reroute), retry with exponential backoff, and observability counters for every failover event.

### 1.3 Control Lane Fault Propagation

```
Process A ──[Data Lane]──► Process B ──[Data Lane]──► Process C
    │                          │                          │
    └──[Control Lane]──────────┴──[Control Lane]──────────┘
                    │
            Fault at Process B:
            1. signal_error(LaneTimeout) → Control Lane
            2. Process A receives Error signal
            3. Process A executes failover strategy
            4. Session teardown propagates to Process C
```

Control signals flow **independently** from data, ensuring fault handling remains responsive even when the Data Lane is under heavy load.

---

## 2. Observability & Latency Tracking

### 2.1 Zero-Instrumentation Design

Every node can be instrumented without manual metric code through annotations:

```rust
#[vil_process]
#[trace_hop]                    // Record latency between node transitions
#[latency_marker("inference")]  // Dashboard label
struct MyProcessor;
```

`#[trace_hop]` and `#[latency_marker]` are **zero-cost annotations**: they add metadata to the IR, and the actual instrumentation code is generated at compile time.

### 2.2 Compiler-Generated Hooks

| Hook Point | Generated Code | Metric |
|------------|---------------|--------|
| Every `port.send()` | `counters.inc_msgs_published()` | Message throughput |
| Every process transition | `latency.record_hop()` | Hop-to-hop latency |
| Every failover execution | `counters.inc_failover_events()` | Failover frequency |
| Every queue push/pop | `gauges.set_queue_depth()` | Queue backpressure |
| Every ownership handoff | `audit.record_transfer()` | Ownership audit trail |

### 2.3 Runtime Metrics Access

```rust
let stats = world.counters_snapshot();
println!("P99 Latency: {} µs", stats.p99_micros());
```

### 2.4 Observer Dashboard

Enable a built-in real-time observability dashboard for any VilApp:

```rust
VilApp::new("my-service")
    .observer(true)   // enables /_vil/dashboard/
    .port(8080)
    .service(service)
    .run()
    .await;
```

Dashboard features:
- **`/_vil/dashboard/`**: Browser-accessible dark-theme SPA with real-time metrics.
- **Endpoint monitoring**: Per-endpoint latency, throughput, and error rate via atomic counters (lock-free).
- **Process topology**: Visual map of ServiceProcess instances and Tri-Lane routes.

Observer REST API:

| Endpoint | Description |
|----------|-------------|
| `GET /vil/observer/metrics` | All endpoint metrics (JSON) |
| `GET /vil/observer/services` | Running services list |
| `GET /vil/observer/plugins` | Plugin registry info |
| `GET /vil/observer/health` | Health check summary |
| `GET /vil/observer/` | Embedded SPA dashboard |

---

## 3. Trust Zones & WASM FaaS

### 3.1 Execution Zone Declaration

When executing third-party or untrusted code, use the Capsule System:

```rust
#[vil_process(zone = NativeCore)]
struct CoreProcessor;          // Full access to SHM, cluster, secrets

#[vil_process(zone = NativeTrusted)]
struct TrustedService;         // Access to SHM, cluster — no secrets

#[vil_process(zone = WasmCapsule)]
struct UserPlugin;             // Sandboxed, no SHM, no cluster

#[vil_process(zone = ExternalBoundary)]
struct ThirdPartyAdapter;      // Most restricted, only I/O
```

### 3.2 Capability Model per Zone (Compile-Time Enforced)

| Capability | NativeCore | NativeTrusted | WasmCapsule | ExternalBoundary |
|------------|-----------|--------------|------------|-----------------|
| `can_emit_lane` | Yes | Yes | Yes | No |
| `can_read_state` | Yes | Yes | No | No |
| `can_use_secret` | Yes | No | No | No |
| `can_access_shm` | Yes | Yes | No | No |
| `can_join_cluster` | Yes | Yes | No | No |
| `can_spawn_process` | Yes | Yes | No | No |
| `can_modify_route` | Yes | No | No | No |

### 3.3 WASM FaaS Runtime

VIL provides real WASM execution via **wasmtime** for Function-as-a-Service workloads:

```rust
use vil_capsule::{CapsuleHost, WasmPool};

// Precompile a WASM module
let host = CapsuleHost::new()?;
let module = host.precompile("pricing.wasm")?;

// Simple i32 call
let result = module.call_i32("calculate_price", 42)?;

// Memory-based call for complex payloads
let output = module.call_with_memory("validate", input_bytes)?;

// Pool for high-throughput FaaS
let pool = WasmPool::new("pricing.wasm", 8)?; // 8 pre-warmed instances
let result = pool.dispatch("calculate_price", payload)?;
```

Three real WASM modules ship with VIL (total: 2,149 bytes compiled):

| Module | Size | Functions | Description |
|--------|------|-----------|-------------|
| `pricing.wasm` | 567B | `calculate_price`, `apply_tax`, `bulk_discount` | Pricing engine |
| `validation.wasm` | 533B | `validate_order`, `validate_age`, `validate_quantity` | Input validation |
| `transform.wasm` | 1,049B | `to_uppercase`, `reverse_bytes`, `count_vowels` | Data transformation |

All modules are `#![no_std]` for ultra-lightweight deployment. `WasmPool` provides pre-warmed instance pooling with round-robin dispatch (~19ns per call overhead).

---

## 4. Sidecar SDK

VIL supports external-process integration via the **Sidecar SDK**, enabling Python, Go, and other languages to participate as VIL Process activities over Unix Domain Sockets with zero-copy SHM data plane.

### 4.1 Python SDK

```python
from vil_sidecar import VilSidecar

app = VilSidecar("fraud-checker")

@app.method("fraud_check")
def fraud_check(request: dict) -> dict:
    score = ml_model.predict(request["features"])
    return {"score": float(score), "is_fraud": score > 0.8}

app.run()
```

### 4.2 Go SDK

```go
app := vil.NewSidecar("ml-engine")
app.Method("predict", func(req vil.Request) vil.Response {
    result := model.Predict(req.JSON())
    return vil.OK(result)
})
app.Run()
```

### 4.3 Production Features

- **ConnectionPool**: Built-in connection pooling with round-robin dispatch over N connections per sidecar.
- **ReconnectPolicy**: Exponential backoff (`base * 2^attempt`, capped at 30s) with deterministic jitter (±25%) to avoid thundering herd.
- **Backpressure**: In-flight tracking — returns error when `max_in_flight` exceeded.

```rust
pub struct PoolConfig {
    pub pool_size: usize,       // default: 4
    pub max_in_flight: u64,     // default: 1000 (0 = unlimited)
}

pub struct ReconnectPolicy {
    pub max_retries: u32,       // default: 10
    pub base_backoff_ms: u64,   // default: 100
    pub max_backoff_ms: u64,    // default: 30000
    pub jitter: bool,           // default: true
}
```

See `examples-sdk/sidecar/` and `examples/021-basic-usage-sidecar-python/` for complete runnable examples.

---

## 5. VIL LSP

VIL ships a **Language Server Protocol** binary (`vil-lsp`) for IDE integration:

- **VS Code**: Install the VIL extension and point it at the `vil-lsp` binary.
- **Diagnostics**: Real-time error checking for VIL macros (`vil_workflow!`, `vil_app!`, semantic type annotations).
- **Completions**: Auto-complete for VIL macro keywords, endpoint attributes, lane types, and ExecClass options.
- **Hover**: Inline documentation for VIL macros, types, and configuration fields.

```bash
# Install the LSP binary
cargo install --path crates/vil_lsp

# VS Code settings.json
{
  "vil.lsp.path": "vil-lsp"
}
```

---

## 6. AI Plugin Infrastructure

VIL provides 51 AI crates following the process-oriented pattern. Each crate has 5 integration layers:

### 6.1 Per-Crate Integration Pattern

```
┌──────────────────────────────────────────────────────┐
│ Layer 5: handlers.rs                                  │
│   REST handlers via vil_server prelude              │
├──────────────────────────────────────────────────────┤
│ Layer 4: plugin.rs                                    │
│   VilPlugin + ServiceProcess                        │
│   .emits() / .faults() / .manages()                  │
├──────────────────────────────────────────────────────┤
│ Layer 3: pipeline_sse.rs                              │
│   SSE factory: xxx_sink(), xxx_source()              │
├──────────────────────────────────────────────────────┤
│ Layer 2: semantic.rs                                  │
│   VilAiEvent / VilAiFault / VilAiState          │
│   Tier B semantic types (JSON-compatible)             │
├──────────────────────────────────────────────────────┤
│ Layer 1: [core logic]                                 │
│   Original algorithms preserved, no VIL dep        │
└──────────────────────────────────────────────────────┘
```

### 6.2 Plugin System Architecture

```rust
VilApp::new("my-ai-service")
    .plugin(LlmPlugin::new())
    .plugin(RagPlugin::new())
    .plugin(AgentPlugin::new())
    .port(8080)
    .run()
    .await;
```

Plugin registration uses Kahn's topological sort for dependency ordering with circular dependency detection.

**VilPlugin trait:**

```rust
pub trait VilPlugin: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str { "" }
    fn capabilities(&self) -> Vec<PluginCapability> { vec![] }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn register(&self, ctx: &mut PluginContext);
    fn health(&self) -> PluginHealth { PluginHealth::Healthy }
    fn shutdown(&self) {}
}
```

### 6.3 Tier A vs Tier B Semantic Types

| Tier | Type System | Transport | Use Case |
|------|-----------|-----------|----------|
| **Tier A** | `#[vil_event]` — fixed-size, VASI-stable, `#[repr(C)]` | SHM zero-copy via ExchangeHeap | Core runtime types |
| **Tier B** | `#[derive(VilAiEvent)]` — any `Serialize` type (String, Vec, HashMap) | JSON over HTTP/SSE/WebSocket | AI domain types |

Tier B intentionally does not use SHM because:
1. AI payloads are dynamic-size (string responses, embedding vectors).
2. JSON serialization is required for SSE streaming to external AI providers.
3. JSON overhead (~1µs) is negligible compared to network latency (~1-50ms) to LLM providers.
4. Tier B still gets semantic classification + lane routing + observability.

### 6.4 Transport Selection

| Scenario | Recommended | Overhead |
|----------|-------------|----------|
| Single-proxy (AI gateway) | `VilApp + SseCollect` (Layer F) | 2-3% |
| Multi-stage pipeline | `vil_sdk + ShmToken` (Layer E) | 2% |

### 6.5 51 AI Crate Categories

| Category | Count | Examples |
|----------|-------|---------|
| Phase A: Core | 3 | `vil_llm`, `vil_rag`, `vil_agent` |
| Tier 0A/0B: Foundation | 9 | `vil_tokenizer`, `vil_embedder`, `vil_vectordb`, `vil_tensor_shm` |
| Tier 1: Enhancement | 6 | `vil_semantic_router`, `vil_prompt_shield`, `vil_reranker` |
| I-Series: Integration | 10 | `vil_crawler`, `vil_graphrag`, `vil_sql_agent`, `vil_ab_test` |
| N-Series: Extensions | 12 | `vil_multimodal`, `vil_federated_rag`, `vil_edge`, `vil_ai_trace` |
| H-Series: Quality | 4 | `vil_chunker`, `vil_guardrails`, `vil_eval` |
| D-Series: Data | 3 | `vil_feature_store`, `vil_streaming_rag`, `vil_model_serving` |
| Additional | 6 | `vil_consensus`, `vil_multi_agent`, `vil_ai_gateway` |

**Total: 51 AI crates**, 164 Tier B semantic types, 1,092+ tests.

---

## 7. SHM Token Architecture

### 7.1 ShmToken vs GenericToken

VIL provides two token types for Tri-Lane transport:

```rust
/// Fixed-size zero-copy token for SHM Tri-Lane transport.
/// 32 bytes, #[repr(C)], is_stable=true → bypasses HashMap store entirely.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShmToken {
    pub session_id: u64,    // 8 bytes
    pub data_offset: u64,   // 8 bytes — offset in SHM ExchangeHeap
    pub data_len: u32,      // 4 bytes
    pub status: u8,         // 1 byte — 0=data, 1=done, 2=error
    pub _pad: [u8; 3],      // 3 bytes alignment
}                           // Total: 32 bytes exactly
```

### 7.2 Performance Comparison

| Metric | GenericToken | ShmToken | Speedup |
|--------|-------------|----------|---------|
| Publish latency (p50) | 850 ns | 95 ns | ~8.9x |
| Publish latency (p99) | 2.1 µs | 180 ns | ~11.7x |
| Throughput (msg/sec) | 1.2M | 8.5M | ~7.1x |
| Memory per message | ~120 bytes | 32 bytes | ~3.8x |
| Allocations per msg | 3 (Arc+HashMap) | 0 (stack copy) | ∞ |

### 7.3 When to Use Which

| Scenario | Recommended |
|----------|-------------|
| High-throughput streaming (SSE, WebSocket) | **ShmToken** |
| Multi-pipeline (fan-out to N consumers) | **ShmToken** |
| Simple request-response | **GenericToken** |
| Cross-host (TCP transport) | **GenericToken** |
| AI plugin communication | **Tier B (JSON)** |

---

## What's New (2026-03-26)

### `.transform()` Callback on `HttpSourceBuilder`

The `HttpSourceBuilder` now supports an inline `.transform()` callback for per-record processing directly within the source node. This eliminates the need for a separate processor stage in simple filter/map pipelines:

```rust
use vil_sdk::http::{HttpSourceBuilder, HttpFormat};

let source = HttpSourceBuilder::new()
    .url("http://localhost:18081/api/v1/credits/ndjson")
    .format(HttpFormat::NDJSON)
    .transform(|line: &[u8]| -> Option<Vec<u8>> {
        // Filter: only forward records with kolektabilitas >= 3
        let record: serde_json::Value = serde_json::from_slice(line).ok()?;
        if record["kolektabilitas"].as_u64()? >= 3 {
            Some(line.to_vec())
        } else {
            None
        }
    })
    .build();
```

Key properties:
- **Per-line for NDJSON**: Callback fires once per newline-delimited record.
- **Per-event for SSE**: Callback fires once per SSE `data:` payload (after dialect parsing).
- **Backpressure-aware**: If the callback is slow, upstream reading is paused (no unbounded buffering).
- **Composable**: Chain `.transform()` with `.json_tap()` — transform runs first, then json_tap extracts the field.

### Tier 2 Pipeline Patterns Documentation

The infrastructure layer now documents **Tier 2 pipeline patterns** used in examples 023-040:

| Pattern | Description | Example |
|---------|-------------|---------|
| **Plugin Composition** | Chain multiple AI plugins (LLM + RAG + Guardrails) in a single pipeline | 026-030 |
| **Multi-Model Router** | Route requests to different LLM providers based on content | 031-033 |
| **A/B Split** | Split traffic between model variants with metric collection | 034-036 |
| **Fallback Chain** | Try primary model, fall back to secondary on failure | 037-038 |
| **Ensemble** | Query multiple models, merge results via consensus scoring | 039-040 |

Each pattern leverages the VIL plugin system (`VilPlugin` trait) for composability and the Tri-Lane mesh for zero-copy inter-plugin communication when services are co-located.

### ShmToken Performance Update

Updated benchmark results for ShmToken on the latest runtime:

| Metric | GenericToken | ShmToken | Speedup |
|--------|-------------|----------|---------|
| Publish latency (p50) | 850 ns | 95 ns | ~8.9x |
| Throughput (msg/sec) | 1.2M | 8.5M | ~7.1x |
| Memory per message | ~120 bytes | 32 bytes | ~3.8x |
| Fan-out to 4 consumers | 300K msg/s | 2.1M msg/s | ~7x |

Fan-out performance is critical for multi-pipeline examples (101-105) where a single source broadcasts to multiple downstream consumers.

---

### Phase 6 Plugin Refactor: All 51 Plugins Use .state() Not .extension()

As of 2026-03-26, the Phase 6 AI crate refactor is **complete**. All 51 AI plugin crates now use the VIL Way plugin registration pattern:

```rust
// Before (Phase 5):
fn register(&self, ctx: &mut PluginContext) {
    ctx.extension(Arc::new(self.config.clone()));  // extension() pattern
}

// After (Phase 6 — VIL Way):
fn register(&self, ctx: &mut PluginContext) {
    ctx.state(self.config.clone());  // state() pattern
}
```

**Refactor metrics:**
- `.state()` in plugin registration: **51/51** crates
- `.extension()` remaining: **0**
- `Extension<T>` in handlers remaining: **0**
- `Json<T>` extractors remaining: **0**

---

*Previous: [Part 4 — Pipeline & HTTP Streaming](./004-VIL-Developer_Guide-Pipeline-Streaming.md)*
*Next: [Part 6 — CLI, Deployment & Best Practices](./006-VIL-Developer_Guide-CLI-Deployment.md)*
