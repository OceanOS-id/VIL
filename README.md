# VIL — Vastar Intermediate Language

A **process-oriented language and framework** hosted on Rust for building zero-copy, high-performance distributed systems.

VIL combines a **semantic language layer** (compiler, IR, macros, codegen) with a **server framework** (VilApp, ServiceProcess, Tri-Lane mesh) — generating all plumbing so developers write only business logic and intent.

```
Developer writes:          VIL generates:
  body: ShmSlice         →  Zero-copy request body via ExchangeHeap
  ctx: ServiceCtx        →  Tri-Lane inter-service messaging
  vil_workflow!           →  Process registration, port wiring, queue plumbing
  .transform(|line|{})   →  Per-record NDJSON/SSE inline processing
  VilResponse::ok(data)  →  SIMD JSON serialization + SHM write-through
```

## Key Differentiators

- **Zero-copy by default** — ShmSlice body extraction, ExchangeHeap, no intermediate buffers
- **Tri-Lane Protocol** — Trigger / Data / Control physically separated (no head-of-line blocking)
- **3 execution modes** — Native Rust (0 overhead), WASM sandbox (~1-5μs), Sidecar any language (~12μs)
- **YAML → Native binary** — write in Python/Go/Java/TypeScript, compile to Rust binary (transpile SDK)
- **51 AI plugin crates** — LLM, RAG, Agent, embeddings, vector DB — all use VIL Way patterns
- **5 SSE dialects** — OpenAI, Anthropic, Ollama, Cohere, Gemini with correct done-signal handling
- **Production config** — profiles (dev/staging/prod), 30+ env vars, SHM pool P99 tuning

## Performance

> Intel i9-11900F (8C/16T), 32GB RAM, Ubuntu 22.04, Rust 1.93.1

| Benchmark | Throughput | Latency (P50) | Notes |
|-----------|-----------|---------------|-------|
| VX_APP HTTP server | **41,000 req/s** | 0.5ms | Pure VIL overhead <1ms |
| AI Gateway (SSE proxy) | **3,600 req/s** | 46ms | 8ms VIL overhead vs direct |
| NDJSON transform (1K rec/req) | **895 req/s** | 183ms | 895K records/s with `.transform()` |
| Multi-pipeline (shared SHM) | **3,700 req/s** | 46ms | ShmToken zero-copy cross-workflow |

Full benchmark with overhead analysis: [examples/BENCHMARK_REPORT.md](examples/BENCHMARK_REPORT.md)

## Quick Start

### Pattern A: HTTP Server (VX_APP)

```rust
use vil_server::prelude::*;

async fn create_task(ctx: ServiceCtx, body: ShmSlice) -> Result<VilResponse<Task>, VilError> {
    let store = ctx.state::<Arc<Store>>()?;
    let input: CreateTask = body.json().map_err(|_| VilError::bad_request("invalid JSON"))?;
    Ok(VilResponse::created(store.insert(input)))
}

#[tokio::main]
async fn main() {
    VilApp::new("tasks")
        .port(8080)
        .profile("prod")
        .service(ServiceProcess::new("tasks")
            .state(store)
            .endpoint(Method::POST, "/tasks", post(create_task)))
        .run().await;
}
```

### Pattern B: Streaming Pipeline (SDK)

```rust
use vil_sdk::prelude::*;

let source = HttpSourceBuilder::new("CreditIngest")
    .url("http://core-banking:18081/api/v1/credits/ndjson?page_size=1000")
    .format(HttpFormat::NDJSON)
    .transform(|line: &[u8]| {
        let r: serde_json::Value = serde_json::from_slice(line).ok()?;
        if r["kolektabilitas"].as_u64()? >= 3 { Some(line.to_vec()) } else { None }
    });

let (_ir, handles) = vil_workflow! {
    name: "NplFilter",
    instances: [sink, source],
    routes: [
        sink.trigger_out -> source.trigger_in (LoanWrite),
        source.data_out  -> sink.data_in      (LoanWrite),
        source.ctrl_out  -> sink.ctrl_in      (Copy),
    ]
};
```

### Pattern C: Custom Code (3 Execution Modes)

```yaml
# Native Rust — compiled in, 0 overhead
endpoints:
  - method: POST
    path: /api/enrich
    handler: enrich_handler
    exec_class: AsyncTask

# WASM — sandboxed, hot-deployable
vil_wasm:
  - name: pricing
    wasm_path: ./wasm-modules/pricing.wasm
    pool_size: 4
    functions:
      - name: calculate_price

# Sidecar — any language (Python, Go, Java)
sidecars:
  - name: ml-scorer
    command: python3
    script: ./sidecars/ml_scorer.py
    methods: [predict, score_batch]
    auto_restart: true
```

### Pattern D: Write in Python, Compile to Native Binary

```python
from vil import VilPipeline

pipeline = VilPipeline("ai-gateway", port=3080)
pipeline.sink(port=3080, path="/trigger")
pipeline.source(url="http://ai-provider:4545/v1/chat", format="sse")
# vil compile --from python --input gateway.py --release → native binary
```

## What's Inside

| Layer | Crates | Purpose |
|-------|--------|---------|
| **Runtime** | vil_types, vil_shm, vil_queue, vil_registry, vil_rt | Zero-copy SHM, SPSC queues, ownership registry |
| **Compiler** | vil_ir, vil_validate, vil_macros, vil_codegen_* | Semantic IR, 10 validation passes, code generation |
| **Server** | vil_server (9 crates) | VilApp, Tri-Lane mesh, 21 middleware, auth, config profiles |
| **Protocol** | vil_grpc, vil_graphql, vil_mq_kafka/nats/mqtt | gRPC, GraphQL, Kafka, NATS, MQTT — all with Tri-Lane bridge |
| **Database** | vil_db_sqlx, vil_db_sea_orm, vil_db_redis, vil_db_semantic | SQLx, SeaORM, Redis, zero-cost semantic layer |
| **AI Plugins** | vil_llm, vil_rag, vil_agent + 48 more | LLM, RAG, Agent, embeddings, vector DB — 51 crates, VIL Way |
| **SDK** | vil_sdk, vil_plugin_sdk, vil_cli | Pipeline SDK, plugin interface, CLI tooling |
| **Execution** | vil_capsule, vil_sidecar | WASM sandbox, sidecar protocol (UDS + SHM) |

**102 crates** | **63 examples** | **5 tiers** | **34 LLM knowledge files**

## Examples (5 Tiers)

| Tier | Count | Pattern | Highlights |
|------|-------|---------|------------|
| **Basic** (001-038) | 38 | VX_APP + SDK | ShmSlice, ServiceCtx, WASM FaaS, sidecar, WebSocket, SSE |
| **Pipeline** (101-107) | 7 | Multi-pipeline | Fan-out, fan-in, diamond, multi-workflow, traced |
| **LLM** (201-206) | 6 | VX_APP + SDK | Chat, multi-model, tools, batch translate, decision routing |
| **RAG** (301-306) | 6 | VX_APP | Vector search, multi-source, hybrid, citation, guardrail |
| **Agent** (401-406) | 6 | VX_APP | Calculator, HTTP fetch, file review, CSV, ReAct, handler+SHM |

```bash
# Run any example
cargo run --release -p vil-basic-hello-server

# Pipeline examples require upstream simulators (for benchmarking & overhead measurement):
#   AI Endpoint:  https://github.com/Vastar-AI/ai-endpoint-simulator    (:4545)
#   Credit Data:  https://github.com/Vastar-AI/credit-data-simulator    (:18081)
cargo run --release -p vil-basic-credit-npl-filter
```

## The 10 Immutable Principles

1. **Everything is a Process** — identity, ports, failure domain
2. **Zero-Copy is a Contract** — VASI/PodLike, ExchangeHeap
3. **IR is the Truth** — macros are frontend, vil_ir is source of truth
4. **Generated Plumbing** — developers never write queue push/pop
5. **Safety Through Semantics** — type system + IR + validation passes
6. **Three Layout Profiles** — Flat, Relative, External
7. **Semantic Message Types** — `#[vil_state/event/fault/decision]`
8. **Tri-Lane Protocol** — Trigger / Data / Control (no head-of-line blocking)
9. **Ownership Transfer Model** — LoanWrite, LoanRead, PublishOffset, Copy
10. **Observable by Design** — `#[trace_hop]`, metrics auto-generated

## VIL Way — 100% Enforced

| VIL Pattern | Replaces | Benefit |
|-------------|----------|---------|
| `body: ShmSlice` | `Json<T>` | Zero-copy via ExchangeHeap |
| `ctx: ServiceCtx` | `Extension<T>` | Tri-Lane context + typed state |
| `body.json::<T>()` | `serde_json` | SIMD JSON (sonic-rs) |
| `VilResponse::ok(data)` | `Json(data)` | SIMD serialization + SHM write-through |

All 51 AI plugins + all 63 examples use these patterns. Zero `Extension<T>`, zero `Json<T>` extractors.

## Documentation

| Guide | File |
|-------|------|
| Architecture Overview | [docs/ARCHITECTURE_OVERVIEW.md](docs/ARCHITECTURE_OVERVIEW.md) |
| Design Principles | [docs/vil/VIL_CONCEPT.md](docs/vil/VIL_CONCEPT.md) |
| Custom Code (Native/WASM/Sidecar) | [docs/vil/CUSTOM_CODE_GUIDE.md](docs/vil/CUSTOM_CODE_GUIDE.md) |
| Developer Guide (6 parts) | [docs/vil/001-VIL-Developer_Guide-Overview.md](docs/vil/001-VIL-Developer_Guide-Overview.md) |
| Server Framework | [docs/vil-server/vil-server-guide.md](docs/vil-server/vil-server-guide.md) |
| API Reference | [docs/vil-server/API-REFERENCE-SERVER.md](docs/vil-server/API-REFERENCE-SERVER.md) |
| Config Reference | [vil-server.reference.yaml](vil-server.reference.yaml) |
| LLM Knowledge Base | [llm_knowledge/](llm_knowledge/index.md) |

## Editor Support

`vil-lsp` provides diagnostics, completions, and hover for VIL macros alongside `rust-analyzer`.

| Editor | Setup |
|--------|-------|
| VS Code | [editors/vscode/](editors/vscode/) |
| Zed | [editors/zed/](editors/zed/) |
| Helix | [editors/helix/](editors/helix/) |
| JetBrains | [editors/jetbrains/](editors/jetbrains/) |

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

## Links

- **Repository:** [github.com/OceanOS-id/VIL](https://github.com/OceanOS-id/VIL)
- **AI Endpoint Simulator:** [github.com/Vastar-AI/ai-endpoint-simulator](https://github.com/Vastar-AI/ai-endpoint-simulator)
- **Credit Data Simulator:** [github.com/Vastar-AI/credit-data-simulator](https://github.com/Vastar-AI/credit-data-simulator)
