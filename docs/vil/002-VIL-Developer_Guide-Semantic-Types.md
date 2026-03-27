# VIL Developer Guide — Part 2: Semantic Types & Memory Model

**Series:** VIL Developer Guide (2 of 8)
**Previous:** [Part 1 — Overview & Architecture](./001-VIL-Developer_Guide-Overview.md)
**Next:** [Part 3 — Server Framework](./003-VIL-Developer_Guide-Server-Framework.md)
**Last updated:** 2026-03-26

---

## 1. Semantic Message Types

Do not use generic `#[message]` if your message plays a specific role in the pipeline. VIL employs semantic macros for compile-time validation:

| Macro | Purpose | Default Lane | Default Memory Class |
| :--- | :--- | :--- | :--- |
| `#[vil_state]` | Mutable state during session lifecycle (State Machine). | Data Lane | `PagedExchange` |
| `#[vil_event]` | Immutable event log entries (e.g., logs, audit trails). | Data/Control Lane | `PagedExchange` |
| `#[vil_fault]` | Structured errors that flow automatically. | Control Lane | `ControlHeap` |
| `#[vil_decision]` | Routing logic restricted to session initiation. | Trigger Lane | `ControlHeap` |

### 1.1 Usage Examples

```rust
#[vil_state]
pub struct SessionProgress {
    pub bytes_processed: u64,
}

#[vil_event]
pub struct TokenGenerated {
    pub session_id: u64,
    pub token_id: u32,
    pub timestamp_ns: u64,
}

#[vil_fault]
pub enum NetworkFault {
    Timeout { domain: String },
    PortBlocked(u16),
}

#[vil_decision]
pub struct RouteDecision {
    pub target_host: HostId,
    pub priority: u8,
}
```

### 1.2 Compile-Time Lane Validation

The `vil_validate` crate enforces these rules at compile time:

| Rule | Condition | Action |
|------|-----------|--------|
| Lane Eligibility | `#[vil_event]` → only Data or Control Lane | Compile error if sent via Trigger Lane |
| Lane Eligibility | `#[vil_decision]` → only Trigger Lane | Compile error if sent via Data Lane |
| Transfer Safety | Type with `Vec<T>` internal → must use `LoanWrite` | Compile error if mode `Copy` |
| Size Warning | Type >64 bytes on Trigger Lane | Warning: consider Data Lane |
| VASI Compliance | Boundary crossing → all fields must be POD/relative | Compile error if `String`/`Box<T>` found |

### 1.3 Pipeline Semantic Types

Pipeline examples should annotate business domain types with VIL semantic macros:

```rust
use vil_sdk::prelude::*;

// State: mutable session data flowing through Data Lane
#[vil_state]
pub struct FilterState {
    pub session_id: u64,
    pub records_processed: u64,
    pub matches_found: u32,
}

// Event: immutable log entry for matches
#[vil_event]
pub struct NplDetected {
    pub session_id: u64,
    pub record_id: u32,
    pub kolektabilitas: u8,
}

// Fault: structured error for Control Lane
#[vil_fault]
pub enum CreditFilterFault {
    UpstreamTimeout { elapsed_ms: u64 },
    InvalidPayload,
}
```

**VASI rule:** Use only `u64`, `u32`, `u16`, `u8`, `bool` — no `String` or `Vec` (those violate zero-copy contract P2).

### 1.4 Example Reference Table

| Macro / Type | Example |
|-------------|---------|
| `#[vil_state]` | 001, 006, 007, 008, 015, 017, 018 |
| `#[vil_event]` | 001, 006, 007, 008, 015, 017, 018 |
| `#[vil_fault]` | 001, 006, 007, 008, 015, 017, 018 |
| `VilResponse<T>` | All server examples (002-005, 009-014, 016) |
| `HandlerResult<T>` + `VilError` | 003, 010, 012, 013, 014 |
| `#[derive(VilModel)]` | 003, 009, 010, 011, 012, 013, 014 |

---

## 2. Server Semantic Types & Macros

VIL provides additional macros and types for server development that enforce "The VIL Way" — typed responses, structured errors, and zero-copy data models.

### 2.1 `VilResponse<T>` — Typed Response Envelope

Replace `Json<serde_json::Value>` with `VilResponse<T>` for typed, contract-safe responses:

```rust
use vil_server::prelude::*;

#[derive(Serialize)]
struct GreetResponse {
    message: String,
    server: &'static str,
}

async fn greet(Path(name): Path<String>) -> VilResponse<GreetResponse> {
    VilResponse::ok(GreetResponse {
        message: format!("Hello, {}!", name),
        server: "vil-server",
    })
}
```

Methods: `VilResponse::ok(data)`, `VilResponse::created(data)`.

### 2.2 `VilError` & `HandlerResult<T>` — Structured Error Handling

Replace manual `(StatusCode, Json<Value>)` error tuples with RFC 7807 Problem Detail errors:

```rust
use vil_server::prelude::*;

async fn get_task(Path(id): Path<u64>) -> HandlerResult<VilResponse<Task>> {
    let task = find_task(id)
        .ok_or_else(|| VilError::not_found(format!("Task {} not found", id)))?;
    Ok(VilResponse::ok(task))
}
```

Factory methods: `.bad_request()`, `.not_found()`, `.unauthorized()`, `.forbidden()`, `.internal()`, `.validation()`, `.rate_limited()`, `.service_unavailable()`.

### 2.3 `#[derive(VilModel)]` — Zero-Copy Data Model

Annotate domain types that may cross SHM boundaries. Generates `from_shm_bytes()` and `to_json_bytes()` using `vil_json` (SIMD-accelerated when `simd` feature is enabled):

```rust
use vil_server::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct Task {
    id: u64,
    title: String,
    done: bool,
}

// Now you can:
let bytes = task.to_json_bytes()?;              // Serialize to Bytes (SHM-ready)
let task: Task = Task::from_shm_bytes(&bytes)?; // Deserialize from SHM
```

### 2.4 `#[derive(DeriveVilError)]` — Custom Error Enums

Define domain-specific error enums that auto-map to HTTP status codes:

```rust
use vil_server::prelude::*;

#[derive(Debug, DeriveVilError)]
enum TaskError {
    #[vil_error(status = 404)]
    NotFound { id: u64 },

    #[vil_error(status = 400)]
    InvalidTitle,

    #[vil_error(status = 500)]
    DatabaseError(String),
}

// Generates: Display, std::error::Error, From<TaskError> for VilError
let err: VilError = TaskError::NotFound { id: 42 }.into();
assert_eq!(err.status, StatusCode::NOT_FOUND);
```

### 2.5 `#[vil_handler]` — Auto-Instrumented Handlers

Wraps async handlers with RequestId injection, tracing spans, and automatic error mapping:

```rust
use vil_server::prelude::*;

#[vil_handler]
async fn get_user(id: Path<u64>) -> Result<User, AppError> {
    let user = db::find_user(*id).await?;
    Ok(user)
}
// Generated wrapper:
//   - Accepts RequestId as first param (auto-injected by Axum)
//   - Opens tracing::info_span!("get_user", request_id = ...)
//   - Ok(data) → VilResponse::ok(data)
//   - Err(e) → VilError via Into<VilError>
```

### 2.6 `#[derive(VilSseEvent)]` — SSE Event Helpers

For Server-Sent Events, derive conversion and broadcast methods:

```rust
use vil_server::prelude::*;

#[derive(Serialize, VilSseEvent)]
#[sse_event(topic = "order_update")]
struct OrderUpdated {
    order_id: u64,
    status: String,
}

// Generated methods:
let event = order.to_sse_event()?;   // → axum::response::sse::Event
order.broadcast(&sse_hub);            // Broadcast to all SSE subscribers
```

### 2.7 `vil_json` — High-Performance JSON

VIL wraps JSON serialization with dual backends:
- **Default:** `serde_json` (zero extra deps)
- **SIMD:** `sonic-rs` (2-3x faster for payloads >256B, enable with `simd` feature)

```rust
use vil_json;

let bytes: bytes::Bytes = vil_json::to_bytes(&data)?;   // SHM-ready
let data: MyType = vil_json::from_slice(&bytes)?;        // Zero-copy path
let val = vil_json!({ "key": "value" });                 // Typed macro
```

---

## 3. Memory Classes & Transfer Modes

Each message can designate its memory location for hardware optimization.

### Memory Classes

| Class | Description | Default for |
|-------|-------------|-------------|
| `PagedExchange` | Default SHM, optimized for inter-process on single host | `#[vil_state]`, `#[vil_event]` |
| `PinnedRemote` | Memory pinned for direct DMA access via RDMA | Explicit only |
| `ControlHeap` | Reserved for small control messages (automatically `Copy`) | `#[vil_fault]`, `#[vil_decision]` |
| `LocalScratch` | Process-local memory (not SHM) | Explicit only |

### Transfer Modes

| Mode | Description |
|------|-------------|
| `LoanWrite` | Borrow SHM memory and write data directly — **Zero Copy** |
| `LoanRead` | Read data directly from SHM pointers |
| `Copy` | Traditional data duplication (only for small control messages) |

### Transfer-Mode Compatibility Matrix (Compile-Time Enforced)

| Memory Class | LoanWrite | LoanRead | Copy | Remote Pull |
|---|---|---|---|---|
| `PagedExchange` | Yes | Yes | No | No |
| `PinnedRemote` | Yes | Yes | No | Yes |
| `ControlHeap` | No | No | Yes | No |
| `LocalScratch` | Yes | Yes | Yes | No |

```rust
// Compile Error Example:
vil_workflow! {
    routes: [
        // HeartbeatSignal is ControlHeap
        // LoanWrite requires PagedExchange or PinnedRemote
        monitor.out -> sink.in (LoanWrite),  // ERROR: ControlHeap
    ]                                         //   incompatible with LoanWrite
}

// Fix:
    routes: [
        monitor.out -> sink.in (Copy),       // OK: ControlHeap + Copy
    ]
```

---

## 4. Session Management & Early-Arrival Buffering

VIL v2 automatically handles race conditions between session registration and initial data arrival through `SessionRegistry`.

- **Early-Arrival Buffer**: If data arrives before session registration, the Kernel holds the message in a temporary buffer (*Pending Slot*) with configurable TTL.
- **Deterministic Teardown**: `Done`/`Abort` signals via Control Lane guarantee resource cleanup (SHM slots) deterministically without waiting for TCP timeouts.
- **Mailbox Isolation**: Each session maintains separate data and control mailboxes, ensuring control signaling remains responsive even under heavy data load.

---

## 5. Ownership Transition Tracking

The IR tracks every message through five lifecycle states:

```
  Allocated ──► Published ──► Received ──► Released ──► Reclaimed
      │                                        │
      │         (leak detection)                │
      └────── if never Published ───────────────┘
              → compile-time warning              → reclaim to pool
```

| State | Meaning | Responsible |
|-------|---------|-------------|
| `Allocated` | Slot allocated in ExchangeHeap | Producer |
| `Published` | Descriptor pushed to queue | Producer (done) |
| `Received` | Consumer popped descriptor | Consumer |
| `Released` | Consumer marked complete (read count) | Consumer |
| `Reclaimed` | Slot returned to allocator | Runtime (automatic) |

The validator detects **memory leaks**: if a message stays `Published` without `Released` for longer than the expected lifetime, the IR produces a diagnostic warning.

---

## 6. VIL IDL & Execution Contract

VIL v2 adopts the principle of **"Ship the Contract"**. Every pipeline produces an **Execution Contract (JSON)** describing the complete topology, data types, and security profile.

### 6.1 Contract Structure

```json
{
  "contract_version": "2.0",
  "pipeline": "CreditFilterPipeline",
  "generated_at": "2026-03-24T10:00:00Z",
  "execution_zone": "NativeTrusted",
  "trust_profile": {
    "can_emit_lane": true,
    "can_read_state": true,
    "can_use_secret": false,
    "can_access_shm": true
  },
  "processes": [...],
  "lanes": [...],
  "routes": [...],
  "failover": {...},
  "observability": {...},
  "messages": [...]
}
```

### 6.2 Contract Consumers

The Execution Contract is consumed by three systems:

| Consumer | How it Uses the Contract |
|----------|------------------------|
| **Neutrino Runtime** | Scheduling: memory class → region sizing, lane config → queue selection, trust zone → capability enforcement |
| **Cluster Orchestrator** | Deployment: host affinity → pod scheduling, failover strategy → replica count, transport → network policy |
| **Monitoring Dashboard** | Auto-discovery: process graph visualization, latency markers → dashboard panel generation, fault events → alert rules |

### 6.3 Contract Benefits

- **Self-Documentation**: Technical pipeline documentation that is always current.
- **Auto-Discovery**: Monitoring dashboards can immediately recognize system structure.
- **Interoperability**: Foundation for generating SDKs in other languages via IDL.

---

## What's New (2026-03-26)

### `#[vil_handler(shm)]` Macro

The `#[vil_handler(shm)]` attribute now auto-generates `ShmSlice` extraction and `ServiceCtx` injection. Instead of manually extracting SHM bytes and looking up state via extensions, the macro handles it:

```rust
use vil_server::prelude::*;

// Before (manual):
async fn process(
    Extension(ctx): Extension<Arc<AppState>>,
    body: Bytes,
) -> VilResponse<Output> {
    let slice = ShmSlice::from_bytes(&body)?;
    // ...
}

// After (with #[vil_handler(shm)]):
#[vil_handler(shm)]
async fn process(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<Output> {
    let state = ctx.state::<AppState>();
    // ShmSlice is auto-extracted from the request body
    // ServiceCtx provides typed access to Tri-Lane context
}
```

The macro generates:
- `ShmSlice` extraction from the request body (replaces manual `Bytes` parsing)
- `ServiceCtx` injection with access to Tri-Lane metadata (session, lane, service name)
- Tracing span with `request_id` and `service_name` fields

### `#[vil_fault]` in Examples

All pipeline examples (001, 006-009, 015, 017-018, 041-046) now use `#[vil_fault]` for structured error types that flow through the Control Lane:

```rust
#[vil_fault]
pub enum PipelineFault {
    UpstreamTimeout { elapsed_ms: u64 },
    InvalidPayload,
    TransformError { stage: &'static str },
}
// Auto-generates: Into<ControlSignal>, signal_error(), control_abort()
```

### `ServiceCtx` as Semantic Tri-Lane Context

`ServiceCtx` replaces the pattern of using `Extension<T>` for accessing shared state within VIL handlers. It provides a typed, semantic context that carries Tri-Lane metadata:

```rust
pub struct ServiceCtx {
    service_name: ServiceName,
    session_id: u64,
    lane: LaneKind,
    // ... internal fields
}

impl ServiceCtx {
    /// Access typed state (replaces Extension<Arc<T>>)
    pub fn state<T: Send + Sync + 'static>(&self) -> &T;

    /// Current service name (injected by VilApp::run())
    pub fn service_name(&self) -> &ServiceName;

    /// Current Tri-Lane session ID
    pub fn session_id(&self) -> u64;
}
```

---

### Phase 6: All 51 AI Crates Now Use Semantic Macros

As of 2026-03-26, all **51 AI plugin crates** use `#[vil_state]`, `#[vil_event]`, and `#[vil_fault]` for their domain types. No AI crate uses plain `#[derive(Serialize, Deserialize)]` without semantic annotation. This ensures:

- Compile-time lane validation for all AI message types
- Automatic transfer mode selection (Tier B JSON for AI payloads)
- Consistent observability categorization across all 51 plugins
- Zero `Extension<T>` or `Json<T>` patterns remaining

---

*Previous: [Part 1 — Overview & Architecture](./001-VIL-Developer_Guide-Overview.md)*
*Next: [Part 3 — Server Framework](./003-VIL-Developer_Guide-Server-Framework.md)*
