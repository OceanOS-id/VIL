# VIL Development Compliance Guide

> Every new crate, connector, trigger, or SDK extension **must** conform to VIL's
> 10 Immutable Principles (VIL_CONCEPT.md). This document defines the compliance
> checklist that all roadmap items must pass before merge.
>
> **Status:** Authoritative — all Phase 0–5 development must reference this document
> **Last updated:** 2026-03-27 (updated: semantic log compliance added)

---

## 1. Process-Oriented Compliance (P1)

Every new crate that exposes runtime behavior must model it as a **VIL Process**.

| Requirement | Check |
|-------------|-------|
| Crate exposes at least one `#[process]` or implements `ServiceProcess` | ☐ |
| Each process has identity (`process_id`), typed ports, failure domain | ☐ |
| No raw `tokio::spawn` for business logic — use VIL process scheduling | ☐ |
| Cleanup policy defined (what happens on crash/abort) | ☐ |
| Execution policy documented (Native / WASM / Sidecar compatibility) | ☐ |

### How this applies to new crates:

**Database connectors** (`vil_db_mongo`, `vil_db_clickhouse`, etc.):
- Connection pool wraps as `ServiceProcess` with health check port
- Query execution is a process stage, not a raw async function
- Pool cleanup on crash is handled via VIL registry

**MQ connectors** (`vil_mq_rabbitmq`, `vil_mq_sqs`, etc.):
- Consumer is a `ServiceProcess` with Trigger/Data/Control ports
- Producer is invoked through Tri-Lane Data Lane, not raw publish calls
- Reconnect/failover is a Control Lane signal, not ad-hoc retry loops

**Triggers** (`vil_trigger_cron`, `vil_trigger_cdc`, etc.):
- Each trigger is a `ServiceProcess` that emits on Trigger Lane
- Trigger lifecycle (start/pause/stop) is managed via Control Lane
- No standalone daemon threads

---

## 2. Zero-Copy Compliance (P2)

| Requirement | Check |
|-------------|-------|
| Hot-path data uses `VSlice<u8>` / `VRef<T>` / `ShmSlice`, not `String`/`Vec<u8>` | ☐ |
| Data allocated in ExchangeHeap, not private heap | ☐ |
| Queue carries descriptors (offsets), not payloads | ☐ |
| Boundary profile documented (which paths are zero-copy, which are Copy fallback) | ☐ |
| No `serde_json::Value` on zero-copy paths | ☐ |

### Boundary classification per crate type:

| Crate Type | Internal Path | External Path | Notes |
|------------|---------------|---------------|-------|
| Database connector | Copy (network boundary) | Copy | DB wire protocol requires serialization — this is acceptable |
| MQ connector | Tri-Lane zero-copy (local) | Copy (network) | Local consumer → SHM, network → Copy |
| Storage connector | Streaming to SHM | Copy (network) | Large objects stream directly to ExchangeHeap pages |
| Trigger | Zero-copy (descriptor) | N/A | Trigger emits lightweight descriptors |
| SDK extension | Full zero-copy | Copy at language boundary | Transpile SDK crosses process boundary |

**Key rule:** Even when external I/O requires Copy, the **internal pipeline path** must remain zero-copy. Data arrives via Copy at the boundary, gets placed into ExchangeHeap, and flows zero-copy from that point onward.

---

## 3. IR & Macro Compliance (P3, P4)

| Requirement | Check |
|-------------|-------|
| If crate defines new semantic types, they emit to `vil_ir` | ☐ |
| No hand-written queue push/pop — use generated plumbing | ☐ |
| No manual offset encode/decode | ☐ |
| No manual metrics instrumentation — VIL generates it | ☐ |
| Any new proc-macro follows: parse → AST → IR → validate → codegen | ☐ |

### For connector crates:

Most connectors don't need new macros. They should:
- Use existing `#[vil_handler]` / `#[vil_endpoint]` for handler functions
- Use `ServiceProcess` registration for runtime integration
- Use `vil_db_semantic` derive macros for DB entities where applicable

---

## 4. Semantic Type Compliance (P6, P7)

| Requirement | Check |
|-------------|-------|
| Messages use correct semantic macro (`#[vil_state]`, `#[vil_event]`, `#[vil_fault]`, `#[vil_decision]`) | ☐ |
| Layout profile declared (Flat / Relative / External) | ☐ |
| No heap types (`String`, `Vec`, `Box`) in Flat/Relative layouts | ☐ |
| Error types use `#[vil_fault]` or `#[connector_fault]`, not plain enums | ☐ |
| Config types may use External profile (acceptable for setup-time data) | ☐ |

### Per crate type:

**Database connector errors:**
```rust
// CORRECT — connector crate (lightweight macro)
#[connector_fault]
pub enum MongoFault {
    ConnectionFailed { uri_hash: u32, reason_code: u32 },
    QueryFailed { collection_hash: u32, reason_code: u32 },
    Timeout { collection_hash: u32, elapsed_ms: u32 },
}
// Generates: Display, error_code(), kind(), is_retryable()

// CORRECT — core/server crate (full IR validation)
#[vil_fault]
pub enum PipelineFault {
    TransferFailed { port: u64, reason_code: u32 },
    LaneTimeout { lane: u64, elapsed_ms: u64 },
}
// Generates: FaultHandler trait, Control Lane integration, IR entry

// INCORRECT — plain enum without derive
pub enum MongoFault {
    ConnectionFailed { uri_hash: u32, reason_code: u32 },
}
// Missing: Display, error_code(), kind() — breaks diagnostics

// INCORRECT — thiserror with String
#[derive(Debug, thiserror::Error)]
pub enum MongoError {
    #[error("Connection failed: {uri}")]
    ConnectionFailed { uri: String },  // heap allocation in fault!
}
```

**MQ message wrappers:**
```rust
// CORRECT — descriptor on Trigger Lane, payload in SHM
#[vil_event(layout = "relative")]
pub struct MqMessage {
    pub topic_hash: u64,
    pub partition: u32,
    pub offset: u64,
    pub payload: VSlice<u8>,    // Points into ExchangeHeap
    pub timestamp_ns: u64,
}

// INCORRECT — heap payload in event
#[vil_event]
pub struct MqMessage {
    pub topic: String,          // Heap!
    pub payload: Vec<u8>,       // Heap!
}
```

---

## 5. Tri-Lane Protocol Compliance (P8)

| Requirement | Check |
|-------------|-------|
| Crate uses Tri-Lane (Trigger/Data/Control) for all inter-process communication | ☐ |
| Trigger Lane: initiates sessions / external events | ☐ |
| Data Lane: hot-path payload delivery | ☐ |
| Control Lane: out-of-band signals (Done/Error/Abort/Pause/Resume) | ☐ |
| Control signals never blocked by Data Lane backpressure | ☐ |

### Tri-Lane mapping for new crate types:

**MQ Connectors:**
| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | New message arrival notification |
| Data | Inbound → VIL | Message payload (via ExchangeHeap) |
| Control | Bidirectional | Ack/Nack/Reject (outbound), Error/Reconnect (inbound) |

**Database Connectors:**
| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Query request descriptor |
| Data | Outbound ← VIL | Query result set (streamed into SHM) |
| Control | Bidirectional | Transaction commit/rollback, connection error |

**Storage Connectors:**
| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Upload/download request |
| Data | Bidirectional | Object content (streamed, chunked into SHM pages) |
| Control | Bidirectional | Progress, error, presigned URL response |

**Triggers:**
| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Outbound → Pipeline | Event fired (cron tick, file change, CDC row, etc.) |
| Data | Outbound → Pipeline | Event payload (if any) |
| Control | Inbound ← Pipeline | Pause/Resume/Stop trigger |

---

## 6. Ownership Transfer Compliance (P9)

| Requirement | Check |
|-------------|-------|
| Transfer mode explicitly declared per data flow | ☐ |
| Linear resources use `ConsumeOnce` | ☐ |
| Shared reads use `ShareRead` (immutable) | ☐ |
| No leaked ownership on crash (registry cleanup) | ☐ |
| Local borrows never cross process boundary | ☐ |

### Common patterns:

| Data Flow | Transfer Mode |
|-----------|--------------|
| MQ message → pipeline | `LoanWrite` (write into SHM) → `PublishOffset` (queue descriptor) |
| DB query result → handler | `LoanWrite` (write result into SHM) → `LoanRead` (handler reads) |
| Storage download → pipeline | `LoanWrite` (stream chunks into SHM pages) |
| Trigger event → pipeline | `Copy` (control-weight) or `LoanWrite` (if payload attached) |
| Config/metadata | `Copy` (setup-time, not hot-path) |

---

## 7. Observability Compliance (P10)

| Requirement | Check |
|-------------|-------|
| `#[trace_hop]` on all inter-process handoffs | ☐ |
| `#[latency_marker]` on critical path operations | ☐ |
| Queue depth gauges auto-generated (no manual) | ☐ |
| Connection pool metrics exposed via `vil_obs` | ☐ |
| Error rates tracked per fault type | ☐ |
| No manual `tracing::info!` for structural metrics — use VIL obs | ☐ |

### Per crate type metrics:

**Database:**
- Query latency histogram (per query type)
- Connection pool: active/idle/waiting counts
- Transaction commit/rollback rate
- Error rate by fault type

**MQ:**
- Message throughput (in/out per second)
- Consumer lag (if applicable)
- Ack/Nack ratio
- Reconnect count

**Storage:**
- Upload/download throughput (bytes/s)
- Operation latency (put/get/delete)
- Error rate by operation type

**Trigger:**
- Fire rate (events/s)
- Missed/skipped events
- Trigger latency (event occurred → VIL received)

---

## 8. Semantic Log Compliance (Phase 0)

Every new crate **must** integrate with `vil_log`. No exceptions.

| Requirement | Check |
|-------------|-------|
| `vil_log = { workspace = true }` in `Cargo.toml` | ☐ |
| No `println!`, `eprintln!`, `dbg!` in production code | ☐ |
| No `tracing::info!` / `log::info!` for VIL-emittable events | ☐ |
| No `env_logger` or `tracing_subscriber::fmt` — use `vil_log::init_logging` | ☐ |
| Auto-emit integrated where applicable (see table below) | ☐ |
| Manual `app_log!` for business logic events only | ☐ |
| `#[vil_fault]` errors emit via auto-emit, not manual log calls | ☐ |

### Auto-Emit Requirements Per Crate Type

| Crate Type | Must Auto-Emit | Log Macro | When |
|------------|---------------|-----------|------|
| **HTTP handler** | AccessLog | `access_log!` | Every request/response (via `#[vil_handler]`) |
| **Database connector** | DbLog | `db_log!` | Every query execution (wrap in timing) |
| **LLM/AI provider** | AiLog | `ai_log!` | Every completion/embed call (wrap in timing) |
| **Message queue** | MqLog | `mq_log!` | Every publish/consume (wrap in timing) |
| **Runtime/process** | SystemLog | `system_log!` | Process register/shutdown/crash |
| **Auth/security** | SecurityLog | `security_log!` | Auth success/fail, rate limit, injection |
| **Storage connector** | DbLog or AppLog | `db_log!` or `app_log!` | Every put/get/delete (wrap in timing) |

### Implementation Pattern

Every new crate must follow this pattern for auto-emit:

```rust
// In the main operation method (e.g., query, publish, put_object):
pub async fn operation(&self, ...) -> Result<T, MyFault> {
    let start = std::time::Instant::now();

    // ... actual operation ...
    let result = self.inner_operation(...).await;

    // Auto-emit log
    let elapsed = start.elapsed();
    db_log!(Info, DbPayload {         // or mq_log!, ai_log!, etc.
        duration_us: elapsed.as_micros() as u32,
        // ... populate fields from context ...
        ..Default::default()
    });

    result
}
```

### Log Field Requirements

**All log payloads must use `u32` hashes for string fields:**

```rust
// CORRECT — hash on hot path, resolve later in drain
use vil_log::dict::register_str;
db_log!(Info, DbPayload {
    table_hash: register_str("users"),    // u32 hash
    query_hash: register_str("SELECT *"), // u32 hash
    ..Default::default()
});

// INCORRECT — String on hot path
tracing::info!(table = "users", query = "SELECT *");  // heap allocation!
```

### LogConfig Thread Hint

New crates that spawn their own thread pools must document the expected thread count
so applications can configure `LogConfig.threads` correctly:

```rust
/// This crate spawns N consumer threads internally.
/// Add N to your `LogConfig.threads` for optimal log ring sizing.
pub struct MyConnector {
    consumer_threads: usize,  // document this
}
```

### Prohibited Patterns

| Pattern | Why | Use Instead |
|---------|-----|-------------|
| `println!("query took {}ms", elapsed)` | Blocks caller, no structure | `db_log!(Info, DbPayload { ... })` |
| `tracing::info!(table = "users")` | String formatting on hot path | `db_log!` with `register_str()` hash |
| `log::error!("connection failed")` | No semantic type, no lane routing | `#[vil_fault]` + auto-emit |
| `eprintln!("debug: {:?}", obj)` | Stderr, no drain routing | `app_log!(Debug, ...)` |
| Manual `metrics::counter!("queries")` | Duplicates VIL obs | Auto-generated by `#[trace_hop]` |

---

## 9. Testing Compliance

| Requirement | Check |
|-------------|-------|
| Unit tests for all public API functions | ☐ |
| Integration tests with Docker container (where applicable) | ☐ |
| At least 1 runnable example in `examples/` | ☐ |
| Benchmark with `vil_server_test::BenchRunner` | ☐ |
| Tri-Lane bridge tested (Trigger/Data/Control paths) | ☐ |
| Crash cleanup tested (kill process, verify no leaked SHM) | ☐ |
| Zero-copy path verified (no unexpected allocations on hot path) | ☐ |

---

## 10. Documentation Compliance

| Requirement | Check |
|-------------|-------|
| `README.md` in crate root | ☐ |
| `description` in `Cargo.toml` | ☐ |
| `license.workspace = true` in `Cargo.toml` | ☐ |
| `authors.workspace = true` in `Cargo.toml` | ☐ |
| Boundary classification documented (zero-copy vs Copy paths) | ☐ |
| Tri-Lane mapping documented | ☐ |
| Example YAML configuration (if configurable) | ☐ |

---

## 11. Crate Structure Template

Every new crate must follow this structure:

```
crates/vil_{name}/
├── Cargo.toml              # workspace inheritance + vil_log dependency
├── README.md               # crate-level docs
├── src/
│   ├── lib.rs              # re-exports, feature gates
│   ├── config.rs           # configuration (External profile OK)
│   ├── client.rs           # core client/driver + auto-emit log in operations
│   ├── bridge.rs           # Tri-Lane bridge adapter
│   ├── types.rs            # #[vil_state], #[vil_event], #[vil_fault] definitions
│   ├── process.rs          # ServiceProcess implementation
│   └── error.rs            # #[vil_fault] error types
├── tests/
│   ├── unit.rs             # unit tests
│   └── integration.rs      # Docker-based integration tests
└── benches/
    └── throughput.rs        # BenchRunner benchmarks
```

**Cargo.toml must include:**
```toml
[dependencies]
vil_log = { workspace = true }    # MANDATORY — semantic log integration
# ... other deps
```

---

## 12. Review Checklist (Pre-Merge)

Before any roadmap crate is merged, reviewer must verify:

- [ ] **P1** — All behavior modeled as VIL Process
- [ ] **P2** — Hot-path is zero-copy; Copy only at network/FFI boundary
- [ ] **P3** — No hand-written plumbing; uses generated code or existing macros
- [ ] **P4** — Developer writes only config + business logic
- [ ] **P5** — Safety enforced by type system, not comments
- [ ] **P6** — Layout profile declared and correct
- [ ] **P7** — Semantic message types used (not generic structs)
- [ ] **P8** — Tri-Lane protocol for all inter-process comm
- [ ] **P9** — Ownership transfer modes explicit, no leaked resources
- [ ] **P10** — Observability auto-generated, no manual metrics
- [ ] **Log** — `vil_log` dependency present, auto-emit integrated
- [ ] **Log** — No `println!`, `tracing::info!`, `log::info!`, `eprintln!`
- [ ] **Log** — String fields use `register_str()` hash, not raw strings
- [ ] **Log** — Correct log macro used (db_log!, mq_log!, ai_log!, etc.)
- [ ] **Log** — Operations wrapped with `Instant::now()` for duration_us
- [ ] **Testing** — Unit + integration + example + benchmark
- [ ] **Docs** — README + Cargo.toml metadata + boundary docs
- [ ] **No `String`/`Vec`/`Box` on hot paths**
- [ ] **No `serde_json::Value` on zero-copy paths**
- [ ] **No `tokio::spawn` for business logic**
- [ ] **No manual `tracing::info!` for structural metrics**
- [ ] **`#[vil_fault]` or `#[connector_fault]` for all error types — no plain enums, no `thiserror`**

---

*This document is the gatekeeper for all VIL roadmap development.
If a crate cannot satisfy these requirements, it must be redesigned before implementation begins.*
