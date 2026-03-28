# VIL Developer Guide — Part 7: Semantic Log System

**Series:** VIL Developer Guide (7 of 8)
**Previous:** [Part 6 — CLI, Deployment & Best Practices](./006-VIL-Developer_Guide-CLI-Deployment.md)
**Next:** [Part 8 — Connectors & Semantic Types](./008-VIL-Developer_Guide-Connectors.md)
**Last updated:** 2026-03-27

---

## 1. Overview

VIL includes `vil_log` — a zero-copy, non-blocking semantic log system purpose-built for high-throughput pipelines. It replaces traditional string-based logging with **typed struct emission** to a lock-free ring buffer.

### Why Not tracing?

| | tracing | VIL Log |
|---|---|---|
| Hot path cost | ~810ns (format + MPMC channel) | ~130ns (memcpy + atomic) |
| Allocation | String buffer per event | Zero (fixed 256B slots) |
| Serialization | On caller thread | Deferred to drain thread (flat types) |
| Blocking | Channel may block | Never blocks (drop + count) |
| Structured types | Generic key-value | 7 typed structs (compile-time checked) |
| Auto-emit | Manual everywhere | Built into `#[vil_handler]`, `vil_llm`, `vil_db_*`, `vil_mq_*` |

For Rust ecosystem compatibility, `VilTracingLayer` bridges `tracing` events into VIL's ring buffer.

### Development vs Production

| Mode | Setup | Latency | When |
|------|-------|---------|------|
| **Development** | Don't call `init_logging()` | ~800ns (tracing fallback) | Debugging, testing, familiar output |
| **Production** | Call `init_logging()` | ~130ns (SPSC ring) | Deployed services, high throughput |

Macros (`app_log!`, `db_log!`, etc.) automatically detect which mode — zero code change needed. See [USAGE.md](../../crates/vil_log/USAGE.md).

---

## 2. Architecture

```
HOT PATH (~130ns per event)              COLD PATH (async tokio task)
───────────────────────────              ────────────────────────────
#[vil_handler] ─► access_log!() ──┐      ┌─► StdoutDrain (pretty/json)
vil_llm        ─► ai_log!()      ──┤      ├─► FileDrain (rolling)
vil_db_*       ─► db_log!()      ──┼─► Striped SPSC Rings ─┼─► ClickHouseDrain (batch)
vil_mq_*       ─► mq_log!()     ──┤   (auto-sized,          ├─► NatsDrain (fan-out)
vil_rt         ─► system_log!()  ──┤    1 ring per CPU core) └─► MultiDrain (N drains)
developer      ─► app_log!()    ──┤
tracing bridge ─► VilTracingLayer┘
```

### Safety & Reliability (v0.2)

| Feature | Description |
|---------|-------------|
| **Collision detection** | 64-bit hash internally, warns if two strings produce same 32-bit truncation |
| **Auto dictionary persist** | Loads `.vil_log_dict.json` on startup, saves on shutdown (ctrl+c/SIGTERM) |
| **Schema versioning** | Resolver dispatches by `(category, version)` — old logs always readable |
| **Zerocopy resolver** | `FromBytes::read_from_prefix()` — no unsafe, no hardcoded byte offsets |
| **FallbackDrain** | Wraps any drain with JSONL file backup after N consecutive failures |
| **Tracing fallback** | When `init_logging()` not called, macros auto-fallback to `tracing::event!` |

### Striped SPSC Rings

VIL auto-detects CPU cores and creates one SPSC ring per core. Threads are assigned via **round-robin** — guaranteed even distribution, zero contention up to core count.

```
LogConfig { threads: Some(4) }  →  4 rings, round-robin assignment
LogConfig { threads: None }     →  auto-detect from available_parallelism()

Thread 0 → Ring 0
Thread 1 → Ring 1
Thread 2 → Ring 2
Thread 3 → Ring 3
Thread 4 → Ring 0 (wrap)
```

---

## 3. Seven Semantic Log Types

Every log event is a 256-byte `LogSlot` = 64-byte `VilLogHeader` + 192-byte typed payload.

### 3.1 VilLogHeader (64 bytes, all events)

```rust
#[repr(C, align(64))]
pub struct VilLogHeader {
    pub event_id: u128,       // ULIDv2 — sortable, unique
    pub trace_id: u64,        // Distributed trace correlation
    pub tenant_id: u64,       // Multi-tenant isolation
    pub process_id: u64,      // Which ServiceProcess emitted
    pub timestamp_ns: u64,    // Nanosecond wall clock
    pub level: u8,            // 0=Trace 1=Debug 2=Info 3=Warn 4=Error 5=Fatal
    pub category: u8,         // 0=Access 1=App 2=System 3=Security 4=Ai 5=Db 6=Mq
    pub subcategory: u8,
    pub version: u8,
    pub service_hash: u32,    // FxHash of service name
    pub handler_hash: u32,    // FxHash of handler name
    pub node_hash: u32,       // FxHash of hostname
}
```

### 3.2 Log Types

| Macro | Payload Struct | Auto-Emitted By | Key Fields |
|-------|---------------|-----------------|------------|
| `access_log!` | `AccessPayload` | `#[vil_handler]` | method, status_code, duration_us, path_hash, client_ip |
| `ai_log!` | `AiPayload` | `vil_llm` providers | model_hash, provider_hash, input/output_tokens, latency_us, cost |
| `db_log!` | `DbPayload` | `vil_db_semantic` | table_hash, query_hash, duration_us, rows_affected, op_type |
| `mq_log!` | `MqPayload` | `vil_mq_nats/kafka/mqtt` | broker_hash, topic_hash, offset, message_bytes, op_type |
| `system_log!` | `SystemPayload` | `vil_rt` | cpu, mem, fd_count, thread_count, event_type |
| `security_log!` | `SecurityPayload` | manual | actor_hash, resource_hash, event_type, outcome, risk_score |
| `app_log!` | `AppPayload` or dynamic KV | manual | code_hash + MsgPack fields (dynamic) or flat struct |

All payload structs are `#[repr(C)]`, ≤192 bytes, `Copy` — pure memcpy into the ring slot.

---

## 4. Usage

### 4.1 Initialization

```rust
use vil_log::prelude::*;
use vil_log::drain::StdoutDrain;
use vil_log::runtime::init_logging;

let config = LogConfig {
    ring_slots: 1 << 20,       // 1M total (divided across stripes)
    level: LogLevel::Info,      // Filter: Debug/Trace discarded (~0.2ns)
    batch_size: 1024,           // Max events per flush
    flush_interval_ms: 100,     // Max ms between flushes
    threads: Some(4),           // 4 worker threads → 4 SPSC rings
};

init_logging(config, StdoutDrain::pretty());
```

### 4.2 Thread Configuration

| Application | `threads` | Why |
|-------------|-----------|-----|
| Web server (tokio) | `None` (auto) | Matches tokio worker count |
| Data pipeline | `Some(8)` or `Some(16)` | Match pipeline parallelism |
| CLI tool | `Some(1)` | Single ring, max per-ring capacity |
| Microservice | `Some(4)` | Typical container allocation |

### 4.3 Auto-Emit (zero developer code)

These log types are emitted automatically by VIL's framework macros:

```rust
// AccessLog — emitted by #[vil_handler] on every HTTP request
// Developer writes:
#[vil_handler]
async fn create_order(body: ShmSlice) -> VilResponse<Order> {
    // ... business logic ...
}
// VIL auto-emits: access_log!(Info, AccessPayload { status, duration, path, ... })

// AiLog — emitted by vil_llm on every LLM call
// Developer writes:
let response = provider.chat(&messages).await?;
// VIL auto-emits: ai_log!(Info, AiPayload { model, tokens, latency, cost, ... })

// DbLog — emitted by vil_db_semantic on every query
// Developer writes:
let user = repo.find_by_id(42).await?;
// VIL auto-emits: db_log!(Info, DbPayload { table, op_type, duration, rows, ... })

// MqLog — emitted by vil_mq_* on every publish/consume
// Developer writes:
nats_client.publish("orders", payload).await?;
// VIL auto-emits: mq_log!(Info, MqPayload { broker, topic, bytes, latency, ... })

// SystemLog — emitted by vil_rt on process lifecycle
// VIL auto-emits on: register_process, shutdown_process, crash_process
```

### 4.4 Manual Emit

```rust
// Dynamic key-value (flexible, ~390ns — MsgPack on hot path)
app_log!(Info, "order.created", {
    order_id: 12345u64,
    amount: 50000u64,
    currency: 360u64,   // ISO numeric
});

app_log!(Warn, "payment.retry", {
    order_id: 12345u64,
    attempt: 3u64,
    reason_code: 4201u64,
});

// Flat struct (maximum speed, ~133ns — pure memcpy)
use vil_log::_emit_typed_log;
_emit_typed_log!(Info, vil_log::types::LogCategory::App, AppPayload {
    code_hash: vil_log::dict::register_str("order.created"),
    kv_len: 0,
    _pad: [0; 2],
    kv_bytes: [0; 184],
});

// Security event
security_log!(Info, SecurityPayload {
    actor_hash: vil_log::dict::register_str("user@example.com"),
    resource_hash: vil_log::dict::register_str("/admin/users"),
    action_hash: vil_log::dict::register_str("delete"),
    event_type: 1,  // authz
    outcome: 1,     // deny
    risk_score: 85,
    ..Default::default()
});
```

### 4.5 Dictionary (hash → string)

Hot-path logs use `u32` hashes instead of strings. Register once, use everywhere:

```rust
use vil_log::dict::register_str;

let path_hash = register_str("/api/orders");   // → u32
let model_hash = register_str("gpt-4o");       // → u32

// Use in payloads:
access_log!(Info, AccessPayload {
    path_hash,
    ..Default::default()
});
```

---

## 5. Drain Backends

### 5.1 Built-in (always available)

```rust
// Stdout — dev mode
StdoutDrain::pretty()   // Multi-line colored
StdoutDrain::compact()  // Single-line colored
StdoutDrain::json()     // JSON Lines (for jq piping)

// Rolling file
FileDrain::new("/var/log/vil", "app", RotationStrategy::Daily, 30)?
FileDrain::new("/var/log/vil", "app", RotationStrategy::Size { max_bytes: 100_000_000 }, 10)?

// Discard (benchmarks)
NullDrain

// Fan-out to multiple drains
MultiDrain::new()
    .add(StdoutDrain::compact())
    .add(FileDrain::new("/var/log/vil", "app", RotationStrategy::Daily, 30)?)
```

### 5.2 Feature-Gated

```toml
# Cargo.toml
vil_log = { workspace = true, features = ["clickhouse-drain"] }
vil_log = { workspace = true, features = ["nats-drain"] }
vil_log = { workspace = true, features = ["all-drains"] }
```

```rust
// ClickHouse — batch INSERT for analytics
use vil_log::drain::{ClickHouseDrain, ClickHouseConfig};

let drain = ClickHouseDrain::new(ClickHouseConfig {
    url: "http://clickhouse:8123".into(),
    database: "vil_logs".into(),
    table: "vil_log".into(),
});

// NATS — cross-host fan-out
// Publishes to: vil.logs.access, vil.logs.ai, vil.logs.db, etc.
use vil_log::drain::{NatsDrain, NatsConfig};

let drain = NatsDrain::new(NatsConfig {
    url: "nats://nats:4222".into(),
    subject_prefix: "vil.logs".into(),
});
```

---

## Schema Backup (Important)

vil_log stores payloads as binary structs, not text. To read old logs in the future:

1. **Back up `.vil_log_dict.json`** — hash→string dictionary (auto-saved on shutdown)
2. **Back up `crates/vil_log/src/types/*.rs`** — struct definitions are the "schema"
3. **Version field** — each LogSlot carries `version: u8` for schema evolution

```bash
# After each release:
cp .vil_log_dict.json log-schema-backup/v0.2.0/
cp crates/vil_log/src/types/*.rs log-schema-backup/v0.2.0/
```

---

## 6. tracing Bridge

Capture Rust ecosystem events (`tokio`, `hyper`, `tower`, etc.) into VIL's ring:

```rust
use vil_log::runtime::init_logging_with_tracing;

// Installs VilTracingLayer as global tracing subscriber.
// All tracing::info!(), tracing::warn!(), etc. flow into VIL ring.
init_logging_with_tracing(config, StdoutDrain::pretty());
```

---

## 7. Benchmark Results

> System: Intel i9-11900F (8C/16T), 32GB RAM, Ubuntu 22.04, Rust 1.93.1
> Payload: 4 equivalent fields per event, 1M events, `--release`

### Single-Thread

| Log Type | ns/event | M ev/s | vs tracing |
|----------|----------|--------|------------|
| tracing (fmt + NonBlocking) | 810 | 1.23 | baseline |
| VIL flat types (access, ai, db, mq, system, security) | 130-178 | 5.3-7.7 | **4.5-6.2x faster** |
| VIL app_log! (flat struct) | 133 | 7.55 | **6.1x faster** |
| VIL app_log! (dynamic MsgPack) | 390 | 2.56 | **2.1x faster** |
| Filtered out (both systems) | 0.2 | ~5000 | parity |

### Multi-Thread (striped SPSC, `threads: 8`)

| Threads | tracing | VIL access_log! | Speedup |
|---------|---------|-----------------|---------|
| 1 | 1.84 M/s | 6.99 M/s | **3.8x** |
| 2 | 3.54 M/s | 10.33 M/s | **2.9x** |
| 4 | 5.13 M/s | 10.51 M/s | **2.0x** |
| 8 | 6.21 M/s | 6.25 M/s | 1.0x |

### File Drain E2E (500K events)

| | tracing (JSON file) | VIL FileDrain |
|---|---|---|
| Emit throughput | 0.82 M/s | **5.47 M/s (6.7x)** |
| File size | 110.5 MB | 59.6 MB (46% smaller) |

---

## 8. Examples

```bash
cargo run -p example-501-villog-stdout-dev           # Stdout pretty output
cargo run -p example-502-villog-file-rolling          # Rolling file drain
cargo run -p example-503-villog-multi-drain           # Multi-drain fan-out
cargo run -p example-504-villog-benchmark-comparison --release  # Full benchmark
cargo run -p example-505-villog-tracing-bridge        # tracing bridge
cargo run -p example-506-villog-structured-events     # All 7 log types
cargo run -p example-507-villog-bench-file-drain --release      # File drain E2E
cargo run -p example-508-villog-bench-multithread --release     # Multi-thread
```

---

*[VIL Community](https://github.com/OceanOS-id/VIL) — Part 7 of 7*
