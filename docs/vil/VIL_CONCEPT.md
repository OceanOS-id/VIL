# VIL Concept — Immutable Design Principles

> This document captures the **non-negotiable design principles** of VIL.
> All implementation decisions, examples, documentation, and community guidance
> must align with these principles. When in doubt, refer here.
>
> **Status:** Authoritative — changes require Core Team consensus
> **Last updated:** 2026-03-26

---

## 1. What VIL IS

VIL is a **process-oriented intermediate language** for building zero-copy, high-performance distributed systems. It is hosted on Rust but exists **above Rust** as a semantic layer with its own:

- Process model
- Message contract system
- Ownership transfer protocol
- Zero-copy discipline (validated at compile-time)
- Observability semantics
- Failure domain model

VIL is NOT a collection of utility crates. It is NOT "Axum with SHM". It is a **language layer** that generates plumbing so developers write only intent and business logic.

---

## 2. The 10 Immutable Principles

### P1: Everything is a Process

All computation units are **processes** in the VIL semantic sense. A process has:
- Identity (process_id)
- Ports (typed input/output)
- Failure domain
- Cleanup policy
- Execution policy
- Observability metadata

Backend mapping (thread, async task, micro-VM) is an implementation detail. The **semantic contract** of "process" is stable.

### P2: Zero-Copy is a Contract, Not a Trick

Zero-copy is not an optimization you sprinkle on hot paths. It is a **system-wide contract** that governs:
- Data layout (VASI/PodLike compliance)
- Allocation location (ExchangeHeap, not private heap)
- Transfer mode (LoanWrite/LoanRead/PublishOffset/Copy)
- Descriptor transport (queue carries offsets, not payloads)
- Boundary legality (compile-time validated)

If a developer uses `String`, `Vec<u8>`, or `serde_json::Value` in a zero-copy path, they are **violating the contract**. VIL must make the correct path easy and the incorrect path hard.

### P3: Macros are Frontend, IR is the Truth

Proc-macros (`#[message]`, `#[process]`, `vil_workflow!`) are **frontend parsers only**. They:
1. Parse declarations
2. Build light AST
3. Submit to semantic IR
4. Call validators
5. Emit generated Rust

The truth lives in `vil_ir`, not in token expansion. This separation ensures:
- IR can be dumped/inspected (JSON/YAML)
- Multiple frontends can target the same IR
- Validation is semantic, not syntactic

### P4: Generated Plumbing, Human-Written Logic

Developers write:
- Message contracts (`#[message]`, `#[vil_state]`, etc.)
- Process definitions (`#[process]`)
- Workflow topology (`vil_workflow!`)
- Business logic (handler functions)

VIL generates:
- Layout validation
- Queue plumbing
- Offset handling
- Cleanup hooks
- Trace/metrics hooks
- Runtime registration
- Interface tx/rx stubs

**Developers should never write queue push/pop, offset encode/decode, ownership tracking, or metrics instrumentation manually.**

### P5: Safety Through Semantics, Not Convention

Safety is not "please be careful". It is enforced by:
- Type system (Vasi, PodLike, LinearResource markers)
- Semantic IR (validated contracts)
- Validation passes (layout, ownership, boundary, queue, cleanup, policy)
- Generated code (correct by construction)
- Runtime invariants (ownership registry, epoch tracking)

### P6: Three Layout Profiles

Every message has exactly one layout profile:

| Profile | Description | Zero-Copy | Use Case |
|---------|-------------|-----------|----------|
| **Flat** | POD/VASI pure, no pointers | Full | Telemetry, counters, state delta |
| **Relative** | Internal references as offsets (`VRef<T>`, `VSlice<T>`) | Full | Frames, documents, variable payloads |
| **External** | Heap types, needs adapter | Copy fallback | Cross-host, FFI, staging |

### P7: Semantic Message Types

Messages are not generic blobs. They carry **semantic roles**:

| Macro | Role | Default Lane | Memory Class |
|-------|------|-------------|-------------|
| `#[vil_state]` | Mutable session state | Data | PagedExchange |
| `#[vil_event]` | Immutable event log | Data/Control | PagedExchange |
| `#[vil_fault]` | Structured error | Control | ControlHeap |
| `#[vil_decision]` | Routing decision | Trigger | ControlHeap |
| `#[message]` | Generic message | Any | Configurable |

Using the correct semantic type enables:
- Compile-time lane validation
- Automatic transfer mode selection
- Observability categorization
- Failure domain routing

### P8: Tri-Lane Protocol

Communication uses three separated lanes to prevent head-of-line blocking:

| Lane | Purpose | Typical Transfer |
|------|---------|-----------------|
| **Trigger** | Start session, handoff from external | LoanWrite |
| **Data** | Hot-path payload | LoanWrite/LoanRead |
| **Control** | Out-of-band signals (Done/Error/Abort) | Copy |

Control signals flow independently from data, ensuring responsive fault handling even under heavy data load.

### P9: Ownership Transfer Model

VIL formalizes six transfer modes:

| Mode | Description | Zero-Copy |
|------|-------------|-----------|
| `LoanWrite` | Borrow SHM slot, write in-place | Yes |
| `LoanRead` | Read from SHM, no clone | Yes |
| `PublishOffset` | Publish descriptor to queue | Yes |
| `Copy` | Traditional copy (control messages) | No |
| `ShareRead` | Shared immutable read | Yes |
| `ConsumeOnce` | Linear resource, exactly-once | Yes |

**Invariants:**
- Shared heap objects never hold pointers to private heap
- One active owner for linear objects at any time
- Hierarchical ownership transfers atomically
- Crash cleanup via registry (no leaked ownership)
- Local borrows never cross process boundary

### P10: Observable by Design

Observability is part of the language contract, not a bolt-on:
- `#[trace_hop]` — automatic hop latency recording
- `#[latency_marker("label")]` — dashboard labels
- Queue depth gauges auto-generated
- Ownership handoff audit trail
- Publish-to-consume latency measurement

Developers do NOT write metrics code. VIL generates it.

---

## 3. The VIL Way — How Developers Should Code

### 3.1 Pipeline Development (vil_sdk)

**Correct:**
```rust
use vil_sdk::prelude::*;

#[vil_state]
pub struct InferenceResult {
    pub session_id: u64,
    pub tokens: VSlice<u8>,      // NOT String
    pub latency_ns: u64,
}

fn main() {
    vil_sdk::http_gateway()       // Layer 1: 5 lines
        .listen(3080)
        .upstream("http://localhost:4545/v1/chat/completions")
        .run();
}
```

**Incorrect:**
```rust
// DON'T: manual reqwest, serde_json, String
let client = reqwest::Client::new();
let resp = client.post(url).json(&body).send().await;
let text: String = resp.text().await?;   // Heap allocation, no zero-copy
```

### 3.2 Server Development (vil_server)

**Correct — using VIL semantic types and extractors:**
```rust
use vil_server::prelude::*;

#[derive(Serialize, Deserialize)]
#[vil_state]                              // Semantic: mutable state
struct Task {
    id: u64,
    title: VSlice<u8>,                      // Zero-copy string
    done: bool,
}

async fn create_task(
    body: ShmSlice,                         // Zero-copy request body
    req_id: RequestId,                      // Auto-propagated
) -> VilResponse<Task> {                  // Typed response envelope
    let input: CreateTaskInput = body.json()?;  // Zero-copy deserialize
    let task = Task { /* ... */ };
    VilResponse::created(task)            // Proper status code
}
```

**Incorrect — plain Axum patterns:**
```rust
// DON'T: bypass VIL extractors
async fn create_task(
    Json(body): Json<serde_json::Value>,    // Heap copy, no SHM
) -> Json<serde_json::Value> {              // No type safety
    Json(serde_json::json!({                // Manual JSON, no contract
        "id": 1,
        "title": "test",                    // String, not VSlice
    }))
}
```

### 3.3 Message Definition

**Correct — zero-copy compliant:**
```rust
#[message(layout = "relative")]
pub struct SensorReading {
    pub sensor_id: u64,
    pub timestamp_ns: u64,
    pub payload: VSlice<u8>,    // Relative-safe
    pub metadata: VRef<SensorMeta>,
}

#[vil_fault]
pub enum SensorFault {
    Timeout { sensor_id: u64, elapsed_ms: u64 },
    Disconnected { sensor_id: u64 },
}
```

**Incorrect — VASI violations:**
```rust
// DON'T: use heap types in messages
#[message]
pub struct SensorReading {
    pub sensor_id: u64,
    pub payload: Vec<u8>,       // VASI violation!
    pub name: String,           // VASI violation!
    pub metadata: Box<Meta>,    // Pointer to private heap!
}
```

### 3.4 Error Handling

**Correct:**
```rust
use vil_server::prelude::*;

async fn get_task(Path(id): Path<u64>) -> HandlerResult<Task> {
    let task = find_task(id)
        .ok_or_else(|| VilError::not_found(format!("Task {} not found", id)))?;
    Ok(VilResponse::ok(task))
}
```

**Incorrect:**
```rust
// DON'T: manual error JSON
async fn get_task(Path(id): Path<u64>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Manual error construction, no RFC 7807, no type safety
}
```

### 3.5 Multi-Service Architecture

**Correct — Process Monolith with Tri-Lane:**
```rust
VilServer::new("platform")
    .port(8080)
    .metrics_port(9090)
    .service_def(
        ServiceDef::new("auth", auth_router)
            .prefix("/auth")
            .visibility(Visibility::Public)
    )
    .service_def(
        ServiceDef::new("orders", orders_router)
            .prefix("/api")
            .visibility(Visibility::Public)
    )
    .service_def(
        ServiceDef::new("payments", payments_router)
            .prefix("/internal/payments")
            .visibility(Visibility::Internal)  // Mesh-only
    )
    .run()
    .await;
```

---

## 4. Implementation Status

### 4.1 What's Fully Realized

| Concept | Implementation | Crate |
|---------|---------------|-------|
| Process as semantic unit | `#[process]`, ProcessIR | vil_macros, vil_ir |
| Semantic message types | `#[vil_state/event/fault/decision]` | vil_macros |
| Zero-copy SHM | ExchangeHeap, ShmSlice, ShmResponse | vil_shm, vil_server_core |
| Tri-Lane protocol | Trigger/Data/Control channels | vil_server_mesh |
| SPSC golden path | SpscRing queue | vil_queue |
| Semantic IR | ProgramIR, MessageIR, ProcessIR | vil_ir |
| 10 validation passes | LayoutLegality through AbiProfile | vil_validate |
| Ownership model | Loaned<T>, Published<T>, LoanedRead<T> | vil_types |
| Relative addressing | VRef<T>, VSlice<T> | vil_types |
| Crash cleanup | Registry, epoch tracking | vil_registry |
| Observable by design | #[trace_hop], #[latency_marker] | vil_macros, vil_obs |
| IR export | JSON/YAML dump | vil_ir, vil_cli |
| C codegen | Header generation from IR | vil_codegen_c |
| WASM capsule trust zones | #[process(zone=WasmCapsule)] | vil_capsule |
| HTTP gateway pipeline | HttpSink/HttpSource/vil_workflow! | vil_sdk, vil_http |
| Server framework | VilServer, 80+ modules, 21 middleware | vil_server |
| Database semantic layer | #[derive(VilEntity)], CrudRepository | vil_db_semantic |
| Protocol support | gRPC, GraphQL, Kafka, MQTT, NATS | vil_grpc, vil_graphql, vil_mq_* |

### 4.2 The Three Runtime Patterns

VIL provides three distinct patterns, each for different use cases:

#### Pattern A: VX_APP (Process-Oriented HTTP Server)

```rust
use vil_server::prelude::*;

async fn create_task(
    ctx: ServiceCtx,   // Tri-Lane context (auto-extracted by VilApp)
    body: ShmSlice,    // Zero-copy body from ExchangeHeap (replaces Json<T>)
) -> VilResponse<Task> {
    let input: CreateTask = body.json().expect("invalid JSON");
    let store = ctx.state::<Arc<Store>>()?;   // replaces Extension<T>
    // ... business logic
    VilResponse::ok(task)
}

VilApp::new("my-service")
    .service(ServiceProcess::new("tasks").state(store).endpoint(POST, "/tasks", post(create_task)))
    .run().await;
```

**Key:** `ShmSlice` (zero-copy body), `ServiceCtx` (Tri-Lane + state), `VilResponse` (SIMD JSON).

#### Pattern B: SDK_PIPELINE (Streaming Pipeline with ShmToken)

```rust
use vil_sdk::prelude::*;

let source = HttpSourceBuilder::new("CreditIngest")
    .url("http://core-banking:18081/api/v1/credits/ndjson?count=1000")
    .format(HttpFormat::NDJSON)
    .transform(|line: &[u8]| {
        let mut r: serde_json::Value = serde_json::from_slice(line).ok()?;
        r["_risk"] = serde_json::json!(if r["kolektabilitas"].as_u64()? >= 3 { "NPL" } else { "OK" });
        Some(serde_json::to_vec(&r).ok()?)
    });

let (_ir, handles) = vil_workflow! {
    name: "Pipeline", instances: [sink, source],
    routes: [ sink.trigger_out -> source.trigger_in (LoanWrite), ... ]
};
source_node.run_worker::<ShmToken>(world, handle);
```

**Key:** `vil_workflow!`, `ShmToken`/`GenericToken`, `.transform()`, Tri-Lane routing.

#### Pattern C: SDK_PIPELINE Multi-Workflow (ShmToken Shared Heap)

```rust
// Two independent pipelines sharing one ExchangeHeap
let world = Arc::new(VastarRuntimeWorld::new_shared()?);

let (_ir_a, handles_a) = vil_workflow! { name: "NplFilter", ... };
let (_ir_b, handles_b) = vil_workflow! { name: "HealthyFilter", ... };

// All workers share same world → ShmToken advantage: O(1) cross-pipeline access
sink_a.run_worker::<ShmToken>(world.clone(), ha);
sink_b.run_worker::<ShmToken>(world.clone(), hb);
```

**Key:** Multiple `vil_workflow!` sharing `VastarRuntimeWorld`, ShmToken for cross-pipeline zero-copy.

### 4.3 Macro Status (Updated 2026-03-25)

| Macro | Purpose | Status |
|-------|---------|--------|
| `#[vil_handler]` | RequestId injection + tracing + error mapping | ✅ Done |
| `#[vil_handler(shm)]` | Auto ShmSlice body + ServiceCtx injection | ✅ Done |
| `#[vil_endpoint]` | Auto JSON body extraction + exec class dispatch | ✅ Done |
| `#[vil_state/event/fault/decision]` | Semantic message types with VASI validation | ✅ Done |
| `#[process]` | Process IR builder | ✅ Done |
| `vil_workflow!` | Declarative pipeline wiring | ✅ Done |
| `vil_app!` | Declarative VX application | ✅ Done |
| `#[vil_service]` | Module-level service definition | ✅ Done |
| `.transform()` | Inline NDJSON/SSE processing on HttpSourceBuilder | ✅ Done |

### 4.4 Examples — 5-Tier Structure (49 Examples)

| Tier | Count | Pattern | Token | Examples |
|------|-------|---------|-------|----------|
| **Tier 1: Basic** | 29 | VX_APP + SDK_PIPELINE | ShmSlice, ShmToken | 001-029 |
| **Tier 2: Pipeline** | 5 | Multi-pipeline | ShmToken | 101-105 (fan-out, fan-in, diamond, multi-workflow) |
| **Tier 3: LLM** | 5 | VX_APP + SDK_PIPELINE | ShmSlice | 201-205 (each unique business logic) |
| **Tier 4: RAG** | 5 | VX_APP | ShmSlice, ServiceCtx | 301-305 (vector, multi-source, hybrid, citation, guardrail) |
| **Tier 5: Agent** | 5 | VX_APP | ShmSlice, ServiceCtx | 401-405 (calculator, HTTP, files, CSV, ReAct loop) |

**100% VIL Way** — zero plain axum patterns (no `Json<T>`, no `Extension<T>`).

### 4.5 AI Plugin Crates — Phase 6 Refactor (Complete)

All 51 AI plugin crates have been refactored to VIL Way:

| Metric | Status |
|--------|--------|
| ServiceCtx in handlers | **51/51** crates |
| ShmSlice for body extraction | **51/51** crates |
| ctx.state::<T>() for state | **51/51** crates |
| #[vil_state/event/fault] | **51/51** crates |
| .state() in plugin registration | **51/51** crates |
| Extension<T> remaining | **0** |
| Json<T> extractors remaining | **0** |
| .extension() remaining | **0** |

---

## 5. Boundary Classification

Not all paths support zero-copy. VIL is honest about this:

| Boundary | Zero-Copy | Transport | Validation |
|----------|-----------|-----------|------------|
| Intra-Process | Full | Direct borrow | Light |
| Inter-Thread (same runtime) | Full | Queue descriptor + SHM | Standard |
| Inter-Process (same host) | Full | Shared memory + offset | Full VASI check |
| Foreign Runtime / WASM | Adapter | Copy/adapted | Boundary check |
| Inter-Host / Network | No | Serialization | External profile |

Developers must know which boundary they're crossing and what transfer modes are legal.

---

## 6. Compilation Pipeline

```
Developer DSL (Rust-hosted)
    │
    ▼
Frontend Parse (vil_macros)
    │
    ▼
Light AST
    │
    ▼
Canonical Semantic IR (vil_ir)
    │
    ▼
Normalization
    │
    ▼
Validation Passes (vil_validate)
    │  LayoutLegality, TransferCapability, BoundaryLegality,
    │  QueueCapability, OwnershipLegality, CleanupObligation,
    │  PolicyCompleteness, AbiProfile, ObservabilityCompleteness
    │
    ▼
Lowering Planner
    │
    ▼
Generated Rust (vil_codegen_rust)
    │
    ▼
rustc
    │
    ▼
Native Binary
```

---

## 7. Design Decisions (Final)

These are settled. Do not re-debate:

1. **Process is a semantic unit, not a synonym for thread.**
2. **IR is the source of truth, not macro expansion.**
3. **SPSC is the golden path; MPMC is opt-in.**
4. **Shared exchange heap is the only arena for zero-copy inter-process transfer.**
5. **Relative addressing is hidden behind safe wrappers.**
6. **Generated plumbing is mandatory; developers don't write pipe fittings.**
7. **Crash cleanup is a core feature, not nice-to-have.**
8. **Interoperability is built through profiles, not "works everywhere" claims.**
9. **Surface syntax may evolve; semantic contracts must not drift.**
10. **Observability is a language contract, not a bolt-on.**

---

## 8. Definition of Done

VIL is "done" for a given feature when:

1. Developer can define process/interface/port/message/policy without manual plumbing
2. Non-VASI/non-layout-safe types are **rejected at compile-time** on zero-copy paths
3. Semantic IR can be dumped and inspected (JSON/YAML)
4. Generated Rust is deterministic and human-readable
5. SHM + ring buffer path sends only descriptors, never payloads
6. Panic/crash cleanup is verified
7. Ownership handoff is auditable
8. Observability metadata is generated automatically
9. At least one `Copy` fallback exists for non-zero-copy boundaries
10. Language contracts remain stable even when runtime backend changes

---

## 9. Benchmark Validation (2026-03-25)

| Example | Pattern | req/s | Notes |
|---------|---------|-------|-------|
| 001 AI Gateway (SSE) | SDK_PIPELINE + ShmToken | **4,142** | 13% overhead vs direct (P50 +4.7ms) |
| 003 Hello Server | VX_APP + ShmSlice | **28,787** | Zero-copy body extraction |
| 007 NPL Filter (NDJSON) | SDK_PIPELINE + .transform() | **927** | 927K records/s (filter kol≥3) |
| 405 Agent ReAct | VX_APP + ShmSlice + ServiceCtx | **54,252** | Multi-tool ReAct loop |

**System:** Intel i9-11900F (8C/16T), 32GB RAM, Ubuntu 22.04, Rust 1.93.1

---

## 10. Project Structure

```
~/Prdmid/vil-project/
├── vil/     101 crates + 49 examples — Open Source (MIT/Apache-2.0)
└── vflow/   Commercial (depends on vil/)
    ├── vflow_server (VLB hot-reload, WASM registry)
    ├── vflow_vrule  (decision rules — planned)
    ├── vflow_vcel   (expression transforms — planned)
    └── docs/ (this file + 6 architectural docs)
```

---

## References

- [001-006 Architectural Implementation](./001-ARCHITECTURAL_IMPLEMENTATION_OVERVIEW.md) — Internal records
- [VIL Developer Guide](../vil/docs/vil/) — Public 6-part guide
- [VIL_CONCEPT.md in vil/](../vil/docs/vil/VIL_CONCEPT.md) — Public version
