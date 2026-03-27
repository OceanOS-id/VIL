# What is VIL?

VIL is a process-oriented language and framework hosted on Rust for building zero-copy, high-performance distributed systems. It combines a semantic language layer (compiler, IR, macros, codegen) with a server framework (VilApp, ServiceProcess, Tri-Lane mesh).

## At a Glance

| Metric | Value |
|--------|-------|
| Rust crates | 102 |
| Examples | 63 (5 tiers) |
| SDK examples | 32 (transpile: Python/Go/Java/TypeScript) |
| Passing tests | 1,425+ |
| Protocols | 7 (REST, SSE, WebSocket, gRPC, Kafka, MQTT, NATS) |
| Execution modes | 3 (Native, WASM, Sidecar) |
| SSE dialects | 5 (OpenAI, Anthropic, Ollama, Cohere, Gemini) |
| Config profiles | 3 (dev, staging, prod) |
| VX_APP throughput | ~41,000 req/s |
| Tri-Lane messaging | 1.9M msg/s |

## Three Application Patterns

### 1. VX_APP (Server)
Process-oriented server with SHM zero-copy IPC (~1-5us per hop).

```rust
VilApp::new("my-server")
    .port(8080)
    .service(ServiceProcess::new("api")
        .endpoint(Method::GET, "/", get(handler)))
    .run().await;
```

### 2. SDK_PIPELINE (Streaming)
HTTP streaming pipeline with SSE/NDJSON via `vil_workflow!`.

```rust
let (_ir, handles) = vil_workflow! {
    name: "Gateway",
    instances: [ sink, source ],
    routes: [ sink.out -> source.in (LoanWrite) ]
};
```

### 3. Multi-Pipeline
Multiple `vil_workflow!` pipelines sharing a common `ExchangeHeap`.

## Key Types

| Type | Purpose |
|------|---------|
| `ShmSlice` | Zero-copy body extraction from HTTP requests |
| `ServiceCtx` | Typed state access with Tri-Lane metadata |
| `VilResponse<T>` | SIMD-accelerated JSON response envelope |
| `VilError` | RFC 7807 structured error with status mapping |
| `ShmToken` | 32-byte fixed-size token for SHM transport |
| `GenericToken` | In-memory token for simple pipelines |

## Example Tiers

| Tier | Range | Focus |
|------|-------|-------|
| 1 | 001-029 | Server, CRUD, mesh, SSE, VilServer, SSE Hub, Macro Demo |
| 2 | 101-105 | Multi-pipeline: fan-out, fan-in, diamond |
| 3 | 201-205 | LLM integration |
| 4 | 301-305 | RAG pipelines |
| 5 | 401-405 | Agent patterns |

## Semantic Macros

```rust
#[vil_state]    // Mutable state -> Data Lane
#[vil_event]    // Immutable log -> Data/Control Lane
#[vil_fault]    // Structured error -> Control Lane
#[vil_decision] // Routing logic -> Trigger Lane
```

> Reference: docs/vil/001-VIL-Developer_Guide-Overview.md
