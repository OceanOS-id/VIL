# VIL — Process-Oriented Intermediate Language

VIL is an **intermediate language** hosted on Rust that makes high-performance, zero-copy distributed systems accessible to teams that are not Rust experts.

VIL sits **above Rust** — developers write intent through semantic macros, and VIL generates all the plumbing: queue wiring, SHM lifecycle, ownership transfer, observability instrumentation.

## What VIL Provides

| Feature | Description |
|---------|-------------|
| **Semantic Type System** | `#[vil_state]`, `#[vil_event]`, `#[vil_fault]`, `#[vil_decision]` |
| **Process Model** | `#[process]`, ports, failure domains, cleanup policies |
| **Zero-Copy Contracts** | VASI validation, `VRef<T>`, `VSlice<T>`, `Loaned<T>` |
| **Workflow DSL** | `vil_workflow!` macro for topology declaration |
| **WASM Sandbox** | `#[vil_wasm]` — zero-plumbing sandboxed execution via wasmtime |
| **Sidecar Integration** | `#[vil_sidecar]` — zero-plumbing polyglot (Python/Go/Java) via SHM+UDS |
| **Transpile SDK** | Write VIL pipelines in Python/Go/Java/TypeScript, compile to native Rust binary |
| **AI Stack** | `LlmRouter`, `RagPipeline`, `Agent` (ReAct), `VectorDB` (HNSW) |
| **WebSocket Semantic** | `#[derive(VilWsEvent)]`, `WsHub` topic-based broadcast |
| **Transfer Modes** | LoanWrite, LoanRead, PublishOffset, Copy, ShareRead, ConsumeOnce |
| **Tri-Lane Protocol** | Trigger/Data/Control separation — no head-of-line blocking |
| **Observability** | `#[trace_hop]`, `#[latency_marker]` — zero manual instrumentation |
| **IR & Validation** | 10 compile-time validation passes, JSON/YAML contract export |

## Three Execution Modes

```rust
// Native — fastest, zero overhead
async fn validate(body: ShmSlice) -> VilResponse<Result> { /* Rust logic */ }

// WASM — sandboxed, hot-deployable (zero plumbing)
#[vil_wasm(module = "pricing")]
fn calculate_price(base: i32, qty: i32) -> i32 { /* business rules */ }

// Sidecar — polyglot, process-isolated (zero plumbing)
#[vil_sidecar(target = "ml-scorer")]
async fn score_fraud(data: &[u8]) -> FraudResult { /* ML scoring */ }
```

Developer writes functions. VIL handles all plumbing — pool management, process spawning, SHM transport, error handling.

## Core Crates (142 total)

| Crate | Purpose |
|-------|---------|
| `vil_types` | Core types, markers (Vasi, PodLike), identity (ProcessId, PortId) |
| `vil_shm` | Shared memory allocator (ExchangeHeap) |
| `vil_queue` | Zero-copy SPSC/MPMC descriptor queues |
| `vil_server` | Process-oriented HTTP server (Axum + VIL runtime) |
| `vil_server_macros` | `#[vil_handler]`, `#[vil_endpoint]`, `#[vil_wasm]`, `#[vil_sidecar]` |
| `vil_capsule` | WASM sandbox (wasmtime) — WasmPool, zero-plumbing bridge |
| `vil_sidecar` | Sidecar protocol (UDS + SHM) — zero-plumbing bridge |
| `vil_llm` | Multi-provider LLM (OpenAI, Anthropic, Ollama), LlmRouter |
| `vil_rag` | RAG pipeline (ingest, chunk, embed, retrieve, generate) |
| `vil_agent` | AI Agent with ReAct loop + ToolRegistry |
| `vil_vectordb` | Native HNSW vector search |
| `vil_orm` | VilEntity derive, VilQuery builder |
| `vil_log` | Semantic log — zero-copy ring buffer, 7 typed categories |
| `vil_sdk` | Pipeline SDK (HttpSource/HttpSink, ShmToken, vil_workflow!) |

## Documentation

| Document | Focus |
|----------|-------|
| [Quick Start](./QUICKSTART.md) | Build your first API in 30 minutes |
| [Developer Guide (11 parts)](./001-VIL-Developer_Guide-Overview.md) | Complete reference |
| [Custom Code Guide](./011-VIL-Developer_Guide-Custom-Code.md) | Native, WASM, Sidecar patterns |
| [VIL Concept](./VIL_CONCEPT.md) | 10 immutable design principles |
| [Architecture Overview](../ARCHITECTURE_OVERVIEW.md) | Layered architecture, 142 crates |
| [Examples](../EXAMPLES.md) | 112 runnable examples |

## Quick Start

```rust
use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    let svc = ServiceProcess::new("hello")
        .endpoint(Method::GET, "/", get(hello))
        .endpoint(Method::POST, "/echo", post(echo));

    VilApp::new("my-app")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}

async fn hello() -> VilResponse<&'static str> {
    VilResponse::ok("Hello from VIL!")
}

async fn echo(body: ShmSlice) -> HandlerResult<VilResponse<serde_json::Value>> {
    let input: serde_json::Value = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;
    Ok(VilResponse::ok(input))
}
```

```bash
cargo run
curl localhost:8080/api/hello/
curl -X POST localhost:8080/api/hello/echo -H 'Content-Type: application/json' -d '{"msg":"test"}'
```
