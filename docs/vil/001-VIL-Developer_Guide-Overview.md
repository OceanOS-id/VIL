# VIL Developer Guide — Part 1: Overview & Architecture

**Series:** VIL Developer Guide (1 of 9)
**Crates:** 130+ | **Tests:** 1,425+ | **Protocols:** 7
**License:** Apache-2.0
**GitHub:** https://github.com/OceanOS-id/VIL
**Last updated:** 2026-03-26

---

## Document Index

This Developer Guide is split into 12 parts for easier navigation:

| # | Document | Scope |
|---|---------|-------|
| **001** | **Overview & Architecture** (this document) | Layered architecture, crate taxonomy, quick start, project stats |
| 002 | [Semantic Types & Memory Model](./002-VIL-Developer_Guide-Semantic-Types.md) | Semantic macros, memory classes, session management, Execution Contract |
| 003 | [Server Framework](./003-VIL-Developer_Guide-Server-Framework.md) | Server macros, VilApp, ServiceProcess, Tri-Lane mesh, DB integration, Observer |
| 004 | [Pipeline & HTTP Streaming](./004-VIL-Developer_Guide-Pipeline-Streaming.md) | `vil_workflow!`, Layer 1/2/3 API, `vil_new_http`, SSE/NDJSON, YAML pipelines |
| 005 | [Infrastructure & Plugins](./005-VIL-Developer_Guide-Infrastructure.md) | Resilience, observability, `#[vil_wasm]`, `#[vil_sidecar]`, LSP, AI plugin system |
| 006 | [CLI, Deployment & Best Practices](./006-VIL-Developer_Guide-CLI-Deployment.md) | CLI reference, Transpile SDK, C interop, health endpoints, deployment, best practices |
| 007 | [Semantic Log System](./007-VIL-Developer_Guide-Semantic-Log.md) | vil_log, SPSC ring buffer, 7 log types, auto-emit, drains |
| 008 | [Connectors & Semantic Types](./008-VIL-Developer_Guide-Connectors.md) | Phase 6 connectors, `#[connector_fault/event/state]`, triggers |
| 009 | [Dual Architecture](./009-VIL-Developer_Guide-Dual-Architecture.md) | VilApp vs ShmToken: when to use which, benchmarks, decision matrix |
| 010 | [Observer Dashboard](./010-VIL-Developer_Guide-Observer-Dashboard.md) | Embedded SPA, metrics, topology, health monitoring |
| **011** | **[Custom Code](./011-VIL-Developer_Guide-Custom-Code.md)** | **Native vs WASM vs Sidecar, `#[vil_wasm]`/`#[vil_sidecar]` macros, decision guide, performance** |

---

## What is VIL?

VIL is a **process-oriented intermediate language** for ultra-low latency distributed systems — combining compile-time semantic validation with a runtime substrate optimized for zero-copy message passing.

Key characteristics:
- **Zero-copy by design**: Shared Memory (SHM) allocator (`ExchangeHeap`) eliminates data copying between co-located services.
- **Tri-Lane Protocol**: Physical separation of Trigger, Data, and Control traffic ensures fault signals are never blocked by data congestion.
- **Semantic-first**: Compile-time macros (`#[vil_state]`, `#[vil_event]`, `#[vil_fault]`, `#[vil_decision]`) enforce message classification and lane eligibility.
- **Process Monolith**: Multi-service in one binary with SHM zero-copy IPC (~1-5µs per hop vs ~500µs-2ms for traditional HTTP IPC).

---

## 1. Layered Architecture

VIL is structured in 15+ layers. Developers typically interact with the top layers while the substrate operates transparently beneath.

```
┌─────────────────────────────────────────────────────────────────────┐
│         Universal YAML Codegen (6 modules)                          │  ← Codegen
│  (Server/DB/MQ/WS-SSE/GraphQL-gRPC/WASM-Sidecar)                  │
├─────────────────────────────────────────────────────────────────────┤
│         Plugin System + AI Infrastructure (52 features)             │  ← Plugin+AI
│  (4-tier plugins, LLM/RAG/Agent, 17 AI crates, 1,092 tests)       │
├─────────────────────────────────────────────────────────────────────┤
│         Transpile SDK (Python/Go/Java/TypeScript → native)          │  ← SDK
│  (32 examples, 4 languages, shorthand DSL, vil compile)          │
├─────────────────────────────────────────────────────────────────────┤
│         VX: Process-Oriented Server (VilApp + ServiceProcess)     │  ← VX
│  (VxKernel, HttpIngress/Egress, VilWsEvent, WsHub)              │
├─────────────────────────────────────────────────────────────────────┤
│         V5.0.1: Semantic Macros + Example Upgrade                   │  ← V5.0.1
│  (VilModel, VilError, vil_handler, VilSseEvent,           │
│   vil_json, bench-json, 18 examples → VIL Way)                │
├─────────────────────────────────────────────────────────────────────┤
│         V10: Adoption (NATS + CLI Init + 12 Templates)             │  ← V10
├─────────────────────────────────────────────────────────────────────┤
│         V9: Protocol (gRPC + Protobuf + Kafka + MQTT)              │  ← V9
├─────────────────────────────────────────────────────────────────────┤
│         V8: Ecosystem (GraphQL + K8s Operator + FFI SDK)            │  ← V8
├─────────────────────────────────────────────────────────────────────┤
│         V7: DB Semantic Layer (Compile-Time IR, Zero-Cost)          │  ← V7
├─────────────────────────────────────────────────────────────────────┤
│         V6: Database ORM Plugins (Pre-compiled Bundles)             │  ← V6
├─────────────────────────────────────────────────────────────────────┤
│         V5: vil-server (Process-Oriented Modular Server)          │  ← V5
│  (Axum + Tower, Service Mesh, SHM Bridge, 21 Middleware, WASM)     │
├─────────────────────────────────────────────────────────────────────┤
│         V4: Community SDK & Tooling                                 │  ← V4
│  (Layer 1/2 API, CLI, YAML Pipeline, Error Catalog, Dockerfile)    │
├─────────────────────────────────────────────────────────────────────┤
│         VIL v2 Semantic Language Layer                            │  ← Wave 1-8
│  (Macros, DSL, Fault Model, Trust Zones, Obs, IR Contract)         │
├─────────────────────────────────────────────────────────────────────┤
│         VIL Phase 1-12 Substrate                                 │  ← Substrate
│  (SHM, Queue, Registry, Tri-Lane, HA, RDMA, Obs)                  │
├─────────────────────────────────────────────────────────────────────┤
│                          Rust + Tokio                               │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.1 Substrate Layer (Phase 1-12)

The foundational infrastructure handling low-level details:
- **Zero-Copy Memory**: Shared Memory allocator (`ExchangeHeap`) with multi-region paged allocation, relative pointers, and adaptive compaction.
- **Lock-Free Queues**: `DescriptorQueue` (MPMC via crossbeam) and `SpscQueue` (SPSC ring buffer, cache-line padded) — only descriptors travel through queues, payloads stay in SHM.
- **Global Routing Table**: SHM-based `#[repr(C)]` atomic records for processes, ports, and routes — accessible cross-process without IPC.
- **Tri-Lane Protocol**: Physical separation of Trigger (session init), Data (payload), Control (lifecycle signals: Done/Error/Abort).
- **High-Availability**: Nanosecond-resolution heartbeat, automated failover via atomic reroute, state sync across nodes.
- **Hardware Acceleration**: RDMA/DPDK support via `VerbsDriver` trait and memory pinning (`mlock`).

### 1.2 Semantic Superlayer (Wave 1-8)

Language-level abstractions above the substrate for safety and productivity:
- **Semantic Type System**: Message classification via `#[vil_state]`, `#[vil_event]`, `#[vil_fault]`, `#[vil_decision]` with compile-time lane validation.
- **Workflow DSL**: Topology declaration via `vil_workflow!` with host affinity, transport selection, and failover intent.
- **Trust Zones**: 4 execution zones (NativeCore > NativeTrusted > WasmCapsule > ExternalBoundary) with compile-time capability enforcement.
- **Observable by Design**: `#[trace_hop]` and `#[latency_marker]` annotations generate instrumentation at compile time — zero manual metric code.
- **Execution Contract**: Every pipeline produces a machine-readable JSON contract consumed by runtime, orchestrator, and monitoring systems.

### 1.3 vil-server (V5/VX)

Process-oriented modular server built on Axum + VIL runtime:
- **120+ modules** across 9 server crates.
- **21 middleware** layers (JWT, RBAC, CSRF, rate limiting, circuit breaker, etc.).
- **Tri-Lane Service Mesh** with SHM zero-copy inter-service communication (<1µs per hop).
- **VX Process-Oriented Architecture**: `VilApp` + `ServiceProcess` + `VxKernel` + `HttpIngress`/`HttpEgress`.
- **Benchmark**: 2.1M req/s GET, 880K ShmSlice ops/s, 1.9M msg/s Tri-Lane.
- See [vil-server Developer Guide](../vil-server/vil-server-guide.md) for full documentation.

### 1.4 Database Integration

Plugin-based database access with provider-neutral semantic layer:
- **Plugins**: sqlx (PostgreSQL/MySQL/SQLite), sea-orm (full ORM), Redis.
- **DB Semantic**: `#[derive(VilEntity)]`, `CrudRepository<T>`, `DatasourceRef`, `DbCapability`.
- **Zero-cost**: All semantic primitives compile away — ~11ns overhead per query (1 vtable call).
- **Provider switch**: Change provider in config + restart. No application code changes for P0 operations.

### 1.5 HTTP Streaming Pipeline (`vil_new_http`)

The sole HTTP streaming crate for building SSE and NDJSON pipelines:
- **`HttpSinkBuilder`**: Accepts incoming HTTP requests (webhook trigger), forwards body upstream.
- **`HttpSourceBuilder`**: Connects to upstream SSE/NDJSON endpoints, streams data back to sink.
- **7 SSE Dialects**: OpenAI, Anthropic, Ollama, Cohere, Gemini, Standard (W3C), Custom.
- **NDJSON Runtime**: Line-by-line newline parsing via `BytesMut` buffer with `FromStreamData` trait.
- **Business-domain examples**: Core Banking credit data streaming (port 18081) for fintech use cases.

> **Note:** `vil_http` has been archived. All HTTP streaming pipelines use `vil_new_http` exclusively.

---

## 2. Crate Taxonomy

### Layer A: Runtime Substrate

| Crate | Description |
|-------|-------------|
| `vil_types` | ID, Descriptor, MessageMeta, SemanticKind, MemoryClass, LaneKind, ControlSignal |
| `vil_shm` | ExchangeHeap, paged allocation, compaction, memory pinning |
| `vil_queue` | MPSC/SPSC queue zero-copy |
| `vil_registry` | Routing table global (SHM), HA state sync |
| `vil_rt` | Kernel runtime, scheduler, session, world/connect |
| `vil_net` | VerbsDriver, RDMA abstraction, mlock |

### Layer B: Semantic Compiler & Validation

| Crate | Description |
|-------|-------------|
| `vil_ir` | IR nodes, PortIR, InterfaceIR, ExecutionContract |
| `vil_macros` | Procedural macros: vil_state/event/fault/decision, trace_hop, vil_workflow! |
| `vil_validate` | Lane legality, memory class compat, VASI validation |
| `vil_codegen_c` | C header generation from IR |
| `vil_codegen_rust` | Tri-lane port generation, auto-wiring |

### Layer C: Trust & Isolation

| Crate | Description |
|-------|-------------|
| `vil_capsule` | WASM capsule host (wasmtime), capability enforcement, WasmPool, WasmFaaSRegistry |

### Layer D: Observability

| Crate | Description |
|-------|-------------|
| `vil_obs` | RuntimeObserver, RuntimeCounters, LatencyTracker |
| `vil_diag` | Diagnostic reporting |

### Layer E: Developer Interface

| Crate | Description |
|-------|-------------|
| `vil_sdk` | Facade for Rust developers (Layer 1/2 API) |
| `vil_new_http` | HTTP streaming pipeline: SSE + NDJSON source/sink, 7 SSE dialects |
| `vil_topo` | YAML topology → workflow code generation |
| ~~`vil_ffi`~~ | *(Removed — FFI runtime replaced by transpile-only SDK)* |
| `vil_cli` | CLI tooling: compile, init, viz, validate, check, wasm, sidecar |
| `vil_viz` | Workflow visualization (6 formats: HTML/SVG/Mermaid/DOT/JSON/ASCII) |

### Layer F: Server Framework (V5/VX)

| Crate | Description |
|-------|-------------|
| `vil_server` | Umbrella re-export crate |
| `vil_server_core` | Axum HTTP engine, VilApp, ServiceProcess, VxKernel, HttpIngress/Egress |
| `vil_server_web` | Valid\<T\>, HandlerError (RFC 7807), HandlerResult\<T\> |
| `vil_server_config` | Multi-source config (YAML/TOML/ENV), profiles |
| `vil_server_mesh` | Tri-Lane SHM service mesh, TCP fallback |
| `vil_server_auth` | JWT, rate limiting, RBAC, CSRF |
| `vil_server_db` | DbPool trait, Transaction wrapper |
| `vil_server_test` | TestClient integration harness |
| `vil_server_macros` | vil_handler, vil_endpoint, vil_service, VilSseEvent, VilWsEvent |

### Layer G-K: Data Infrastructure & Ecosystem

| Layer | Crates | Description |
|-------|--------|-------------|
| G (V6) | `vil_db_sqlx`, `vil_db_sea_orm`, `vil_db_redis` | Database plugins |
| H (V7) | `vil_db_semantic`, `vil_db_macros`, `vil_cache` | DB semantic layer |
| I (V8) | `vil_graphql`, `vil_operator` | Ecosystem (GraphQL, K8s) |
| J (V9) | `vil_grpc`, `vil_server_format`, `vil_mq_kafka`, `vil_mq_mqtt` | Protocol |
| K (V10) | `vil_mq_nats`, CLI templates | Adoption tooling |

### Additional Crates

| Crate | Description |
|-------|-------------|
| `vil_sidecar` | Sidecar Protocol (UDS + SHM zero-copy IPC) |
| `vil_script_js` | Sandboxed JavaScript runtime (boa_engine) |
| `vil_script_lua` | Sandboxed Lua runtime (mlua) |
| `vil_lsp` | Language Server (diagnostics, completions, hover) |
| `vil_observer` | Observer Dashboard (embedded SPA) |
| `vflow_server` | VLB artifact provisioning runtime |

---

## 3. Quick Start

### 3.1 Minimal Server (VilApp)

```rust
use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    let service = ServiceProcess::new("hello")
        .endpoint(Method::GET, "/", get(hello))
        .endpoint(Method::GET, "/greet/:name", get(greet));

    VilApp::new("hello-server")
        .port(8080)
        .service(service)
        .run()
        .await;
}

async fn hello() -> &'static str { "Hello from VIL!" }

async fn greet(Path(name): Path<String>) -> VilResponse<GreetResponse> {
    VilResponse::ok(GreetResponse {
        message: format!("Hello, {}!", name),
        server: "vil-server",
    })
}

#[derive(Serialize)]
struct GreetResponse {
    message: String,
    server: &'static str,
}
```

### 3.2 Minimal Pipeline (Layer 1 Gateway)

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

### 3.3 CLI Scaffolding

```bash
vil new my-project --template stream-filter
cd my-project
vil run --mock   # Run with built-in mock server
```

---

## 4. Project Statistics

| Metric | Value |
|--------|-------|
| Rust crates | 142 |
| Architecture layers | 15+ |
| Native examples | 93 (8 tiers: Basic/Pipeline/LLM/RAG/Agent/VilLog/DB/Trigger) |
| SDK examples | 8 languages (Python/Go/TypeScript/Java/C#/Kotlin/Swift/Zig) |
| vflow examples | 9 |
| Passing tests | 1,425+ |
| Protocols | 12 (REST, SSE, WebSocket, gRPC, Kafka, MQTT, NATS, RabbitMQ, SQS, SOAP, Modbus, OPC-UA) |
| YAML Codegen modules | 6 |
| Code execution modes | 3 macros: native, `#[vil_wasm]`, `#[vil_sidecar]` |
| AI features | LlmRouter, RagPipeline, Agent (ReAct), VectorDB (HNSW) |
| HTTP streaming crate | `vil_new_http` (SSE + NDJSON, 7 dialects) |
| Benchmark (HTTP) | 2.1M req/s |
| Benchmark (ShmSlice) | 880K ops/s |
| Benchmark (Tri-Lane) | 1.9M msg/s |

---

## 5. Example Categories

| Category | Examples | SSE Source |
|----------|---------|------------|
| **Server (REST/CRUD)** | 002, 003, 005, 009-014, 016 | N/A |
| **Fintech SSE (Core Banking)** | 004, 006, 007, 008 | Port 18081 (`/api/v1/credits/stream`) |
| **AI SSE (LLM/RAG/Agent)** | 001, 015, 017-018, 023-040 | Port 4545 (`/v1/chat/completions`) |
| **A/B Testing** | 019 | N/A |
| **WASM/Sidecar/Hybrid** | 020-022 | N/A |
| **Plugin Composition** | 026-040 | Port 4545 |

---

## What's New (2026-03-26)

### Phase 6 AI Crate Refactor — COMPLETE

All **51/51 AI plugin crates** are now fully VIL Way compliant:
- `ServiceCtx` in all handlers (zero `Extension<T>` remaining)
- `ShmSlice` for all body extraction (zero `Json<T>` extractors remaining)
- `.state()` in all plugin registrations (zero `.extension()` remaining)
- `#[vil_state]`, `#[vil_event]`, `#[vil_fault]` in all semantic types

### Crate & Example Growth

- **101 crates** total (`vil_ffi` removed — FFI runtime replaced by transpile-only SDK).
- **49 native examples** organized in a **5-tier structure**:
  - **Tier 1 (001-029):** Basic usage — server, CRUD, mesh, pipeline, VilServer, SSE Hub, Macro Demo
  - **Tier 2 (101-105):** Multi-pipeline — fan-out, fan-in, diamond, multi-workflow
  - **Tier 3 (201-205):** LLM integration (each unique business logic)
  - **Tier 4 (301-305):** RAG pipelines (vector, multi-source, hybrid, citation, guardrail)
  - **Tier 5 (401-405):** Agent patterns (calculator, HTTP, files, CSV, ReAct loop)
- **3 new examples:** 027 VilServer, 028 SSE Hub, 029 Macro Demo (Tier 1 basic now 29 examples)

### SDK: Transpile Only (FFI Removed)

The FFI runtime (`vil_ffi`, ctypes/cgo/JNI bindings) has been removed. The SDK is now **transpile-only**:
- `vil compile --from python|go|java|typescript` produces a native Rust binary
- No FFI overhead, no runtime dependencies
- 32 SDK transpile examples across 4 languages

### 2-Tier Project Structure

The VIL ecosystem is now split into two complementary repositories:

| Repository | Purpose | Contents |
|-----------|---------|----------|
| **vil** | Core framework + examples | 101 crates, 49 examples, SDK, CLI, benchmarks |
| **vflow-server** | VLB artifact provisioning | Runtime server for deploying compiled VIL pipelines |

Sync scripts (`sync-to-vil.sh`, `sync-to-vflow.sh`) keep shared crates in lockstep between the two repos.

### Benchmark Highlights

| Metric | Previous | Current |
|--------|----------|---------|
| HTTP GET throughput | 2.0M req/s | 2.1M req/s |
| ShmSlice ops | 860K ops/s | 880K ops/s |
| Tri-Lane messaging | 1.8M msg/s | 1.9M msg/s |

### Other Changes

- axum unified to **0.7** across all crates (removed legacy 0.6 dependency).
- `VilResponse` now uses `vil_json` (SIMD-accelerated) instead of `serde_json` for response serialization.
- `ServiceCtx` introduced as the semantic Tri-Lane context type (replaces `Extension<T>` pattern).
- `HttpSourceBuilder::transform()` enables inline NDJSON/SSE processing without a separate processor node.
- `SseHub` / `SseEvent` exported in prelude for server-side SSE broadcasting.
- Codegen now generates VIL Way handlers: `ctx: ServiceCtx`, `body: ShmSlice`.

---

*Next: [Part 2 — Semantic Types & Memory Model](./002-VIL-Developer_Guide-Semantic-Types.md)*
