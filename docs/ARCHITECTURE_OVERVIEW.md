# VIL Architecture Overview

**130+ crates** | **83 examples** | **1,425+ tests** | **7 protocols** | **3 execution modes**
**License:** MIT OR Apache-2.0 | **Repository:** [github.com/OceanOS-id/VIL](https://github.com/OceanOS-id/VIL)

---

## 1. Layered Architecture (15 Layers)

Each layer builds on the one below without modifying its contracts.

```
┌─────────────────────────────────────────────────────────────────┐
│  Universal YAML Codegen (6 modules)                              │  Codegen
│  (Server / DB / MQ / WS-SSE / GraphQL-gRPC / WASM-Sidecar)      │
├─────────────────────────────────────────────────────────────────┤
│  Plugin System + AI Infrastructure (51 crates, 164 types)        │  AI
│  (4-tier: Native → Process → WASM → Sidecar)                    │
├─────────────────────────────────────────────────────────────────┤
│  Transpile SDK (Python / Go / Java / TypeScript → native)        │  SDK
├─────────────────────────────────────────────────────────────────┤
│  VX: Process-Oriented Server (VilApp + ServiceProcess)         │  VX
│  (VxKernel, HttpIngress/Egress, Tri-Lane Mesh)                 │
├─────────────────────────────────────────────────────────────────┤
│  Configuration (Profiles, YAML, ENV, ShmPoolConfig)            │  Config
├─────────────────────────────────────────────────────────────────┤
│  Semantic Macros (VilModel, vil_handler, vil_json)             │  Macros
├─────────────────────────────────────────────────────────────────┤
│  Protocol (gRPC, GraphQL, Kafka, MQTT, NATS)                   │  Protocol
├─────────────────────────────────────────────────────────────────┤
│  Database (sqlx, sea-orm, Redis, Semantic Layer)               │  DB
├─────────────────────────────────────────────────────────────────┤
│  vil-server (Axum + Tower, 21 Middleware, Service Mesh)        │  Server
├─────────────────────────────────────────────────────────────────┤
│  Community SDK & Tooling (CLI, YAML Pipeline, Error Catalog)   │  Tooling
├─────────────────────────────────────────────────────────────────┤
│  VIL Semantic Language Layer (Macros, DSL, Trust Zones)        │  Language
├─────────────────────────────────────────────────────────────────┤
│  Runtime Substrate (SHM, Queue, Registry, Tri-Lane, HA)        │  Substrate
├─────────────────────────────────────────────────────────────────┤
│  Rust + Tokio                                                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Crate Taxonomy (11 Layers, 130+ Crates)

### Layer A: Runtime Substrate

| Crate | Purpose |
|-------|---------|
| `vil_types` | IDs, Descriptors, SemanticKind, MemoryClass, LaneKind, ControlSignal |
| `vil_shm` | ExchangeHeap — zero-copy shared memory, paged allocation, compaction |
| `vil_queue` | MPSC/SPSC zero-copy queues over SHM |
| `vil_registry` | Global atomic routing table, HA state sync |
| `vil_rt` | Kernel runtime, scheduler, session management, world/connect |
| `vil_net` | VerbsDriver, RDMA abstraction, memory pinning |

### Layer B: Semantic Compiler

| Crate | Purpose |
|-------|---------|
| `vil_ir` | Intermediate Representation — PortIR, InterfaceIR, ExecutionContract |
| `vil_macros` | Procedural macros: `#[vil_state/event/fault/decision]`, `vil_workflow!` |
| `vil_validate` | 10 validation passes — lane legality, VASI layout, memory class |
| `vil_codegen_rust` | Tri-Lane port generation, auto-wiring from IR |
| `vil_codegen_c` | C header generation for FFI |

### Layer C: Trust & Isolation

| Crate | Purpose |
|-------|---------|
| `vil_capsule` | WASM sandbox (wasmtime) — WasmPool, WasmFaaSRegistry, Level 1 zero-copy |

### Layer D: Observability

| Crate | Purpose |
|-------|---------|
| `vil_obs` | RuntimeObserver, RuntimeCounters, LatencyTracker |
| `vil_diag` | Diagnostic reporting |

### Layer E: Developer Interface

| Crate | Purpose |
|-------|---------|
| `vil_sdk` | Pipeline SDK — HttpSource/HttpSink, ShmToken, vil_workflow! macro |
| `vil_new_http` | HTTP streaming: SSE + NDJSON source/sink, 7 SSE dialects, SIMD JSON |
| `vil_cli` | CLI: `vil compile`, `vil run`, `vil init`, `vil viz`, `vil doctor` |
| `vil_viz` | Workflow visualization (HTML, SVG, Mermaid, DOT, JSON, ASCII) |
| `vil_plugin_sdk` | Stable community plugin interface — PluginBuilder, testing harness |

### Layer F: Server Framework

| Crate | Purpose |
|-------|---------|
| `vil_server` | Umbrella re-export crate |
| `vil_server_core` | VilApp, ServiceProcess, VxKernel, HttpIngress/Egress, 21 middleware |
| `vil_server_config` | Profiles (dev/staging/prod), YAML + ENV, 30+ env vars |
| `vil_server_mesh` | Tri-Lane SHM service mesh, TCP fallback, scatter-gather, DLQ |
| `vil_server_auth` | JWT, rate limiting, RBAC, CSRF, circuit breaker |
| `vil_server_macros` | `#[vil_handler]`, `#[vil_endpoint]`, `VilModel`, `VilSseEvent`, `VilWsEvent` |
| `vil_server_web` | `Valid<T>`, `HandlerResult<T>`, OpenAPI generation |
| `vil_server_db` | DbPool trait, Transaction wrapper |
| `vil_server_test` | TestClient, BenchRunner |

### Layer G: Database Plugins

| Crate | Purpose |
|-------|---------|
| `vil_db_sqlx` | PostgreSQL/MySQL/SQLite, MultiPoolManager, per-query metrics |
| `vil_db_sea_orm` | Full ORM with migrations |
| `vil_db_redis` | Redis cache, JSON helpers, TTL eviction |
| `vil_db_semantic` | `#[derive(VilEntity)]`, `CrudRepository<T>`, `DatasourceRegistry` — zero-cost |
| `vil_cache` | VilCache trait, SHM + Redis backends |

### Layer H: Protocol & Messaging

| Crate | Purpose |
|-------|---------|
| `vil_grpc` | gRPC via tonic, 5-line GrpcGatewayBuilder |
| `vil_graphql` | async-graphql, auto-generated schema, CrudResolver, subscriptions |
| `vil_mq_nats` | NATS Core/JetStream/KV, Tri-Lane bridge |
| `vil_mq_kafka` | Kafka producer/consumer, Tri-Lane bridge |
| `vil_mq_mqtt` | MQTT client, QoS, Tri-Lane bridge |
| `vil_server_format` | FormatResponse — auto JSON/Protobuf content negotiation |

### Layer I: AI Plugin Infrastructure (51 crates)

| Tier | Count | Crates |
|------|-------|--------|
| Official | 3 | `vil_llm`, `vil_rag`, `vil_agent` |
| Core AI | 5 | Embedder, tokenizer, inference, prompts, output parser |
| AI Infra | 15 | Cache, proxy, guardrails, routing, tracing, gateway |
| RAG | 8 | Vector DB, chunker, reranker, graph RAG, federated, streaming |
| Agent & Workflow | 10 | Multi-agent, SQL agent, eval, synthetic data, A/B test |
| Multimodal & Edge | 12 | Vision, audio, doc parsing, edge, quantized |

All 51 crates follow the 5-layer VIL pattern: semantic types → SSE pipeline → VilPlugin → handlers → core logic.

### Additional

| Crate | Purpose |
|-------|---------|
| `vil_sidecar` | Sidecar protocol (UDS + SHM zero-copy IPC), reconnect, failover |
| `vil_operator` | Kubernetes CRD operator |
| `vil_lsp` | Language Server — diagnostics, completions, hover for VIL macros |
| `vil_observer` | Observer Dashboard (embedded SPA) |
| `vil_connector_macros` | Lightweight proc-macro: #[connector_fault/event/state] for connector crates |

---

## 3. Three Execution Modes

| Mode | ExecClass | Overhead | Isolation | Hot-Deploy | Languages |
|------|-----------|----------|-----------|------------|-----------|
| **Native** | `Native` | 0 | None | No (recompile) | Rust |
| **WASM** | `WasmFaaS` | ~1-5μs | Memory sandbox | Yes | Rust → .wasm, AssemblyScript |
| **Sidecar** | `SidecarProcess` | ~12μs | Full process | Yes | Python, Go, Java, TypeScript, any |

Failover chain: Native → WASM → Sidecar (degrade performance, preserve availability).

Custom code guide: [CUSTOM_CODE_GUIDE.md](./vil/CUSTOM_CODE_GUIDE.md)

---

## 4. Configuration Architecture

```
Code Default → YAML (vil-server.yaml) → Profile (dev/staging/prod) → ENV (VIL_*)
```

| Profile | SHM | Logging | DB Pool | Admin | Security |
|---------|-----|---------|---------|-------|----------|
| dev | 8MB | debug, text | 5 conn | all on | off |
| staging | 64MB | info, json | 20 conn | selective | rate limit |
| prod | 256MB | warn, json | 50 conn | all off | hardened |

30+ environment variable overrides. Reference: `vil-server.reference.yaml`

---

## 5. SSE Dialect System

| Dialect | Done Signal | JSON Tap | Provider |
|---------|-------------|----------|----------|
| OpenAI | `data: [DONE]` | `choices[0].delta.content` | OpenAI, Azure, Mistral |
| Anthropic | `event: message_stop` | `delta.text` | Claude |
| Ollama | `"done": true` | `message.content` | Ollama (local) |
| Cohere | `event: message-end` | `text` | Cohere |
| Gemini | TCP EOF | `candidates[0].content.parts[0].text` | Google Gemini |

---

## 6. Performance

> Intel i9-11900F (8C/16T), 32GB RAM, Ubuntu 22.04, Rust 1.93.1

| Metric | Result |
|--------|--------|
| VX_APP HTTP throughput | ~41,000 req/s (P50 0.5ms) |
| NDJSON transform (1K rec/req) | ~895 req/s = 895K records/s |
| AI Gateway SSE (via VIL) | ~3,600 req/s |
| VIL routing overhead | ~8ms fixed |
| ShmSlice throughput | 860K ops/s |
| Tri-Lane mesh latency | <1μs |
| DB semantic overhead | ~11ns per query |

Full benchmark: [examples/BENCHMARK_REPORT.md](../examples/BENCHMARK_REPORT.md)

---

## 7. Document Index

| Document | Focus |
|----------|-------|
| [VIL Concept](./vil/VIL_CONCEPT.md) | 10 immutable design principles |
| [Custom Code Guide](./vil/CUSTOM_CODE_GUIDE.md) | Native / WASM / Sidecar execution modes |
| [Developer Guide (8 parts)](./vil/001-VIL-Developer_Guide-Overview.md) | Complete language + framework reference |
| [vil-server Guide](./vil-server/vil-server-guide.md) | Server framework reference |
| [API Reference](./vil-server/API-REFERENCE-SERVER.md) | Per-module API documentation |
| [Quick Start](./QUICK_START.md) | Getting started |
| [Examples](./EXAMPLES.md) | 83 runnable examples |
| [Changelog](./CHANGELOG.md) | Release history |

---

*[VIL Community](https://github.com/OceanOS-id/VIL) — Last updated: 2026-03-27*
