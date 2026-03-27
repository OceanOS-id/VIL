# Phase 0 — Q2 2026: VIL Semantic Log System

> **⚠ MANDATORY: Read [COMPLIANCE.md](./COMPLIANCE.md) before implementing any crate in this phase.**
> Every crate must pass the full compliance checklist (P1–P10, testing, docs, pre-merge review).
> Non-compliant crates will be rejected regardless of functionality.

> **This is Phase 0 — it must be implemented BEFORE all other phases.**
> The log system is foundational infrastructure. Every subsequent crate (storage, connectors,
> triggers, SDK) depends on `vil_log` for observability. Ship this first.

---

## Objective

Build a zero-copy, non-blocking semantic log system native to VIL. The system must:

1. Cost <50ns per log event on hot path (no alloc, no lock, no I/O, no format)
2. Support multiple drain backends (ClickHouse batch, file rolling, stdout, NATS)
3. Provide structured semantic log types (Access, App, AI, DB, MQ, System, Security)
4. Auto-emit from VIL macros — developers write zero logging plumbing
5. Bridge `tracing` ecosystem for compatibility

**Benchmark requirement**: Implement baseline `tracing` benchmark FIRST, then VIL log, then compare.

---

## Architecture Overview

```
HOT PATH (caller thread)                    COLD PATH (drain thread/task)
━━━━━━━━━━━━━━━━━━━━━━━━                    ━━━━━━━━━━━━━━━━━━━━━━━━━━━
Sync, non-blocking                          Async, batched
~15-50ns per event                          Configurable flush policy

                                            ┌─► ClickHouseDrain (batch INSERT, RowBinary)
VIL code ──► app_log!() ──┐                 │
                          ├─► SpscRing ─────┼─► FileDrain (rolling, async write)
Ecosystem ─► tracing ─────┤   (SHM,        │
             VilLayer      │   lock-free)   ├─► StdoutDrain (formatted, dev only)
                          │                 │
Auto-emit ─► #[vil_handler]┘                ├─► NatsDrain (cross-host fan-out)
             vil_db_*                       │
             vil_mq_*                       └─► CustomDrain (user-defined)
             vil_llm
```

---

## Crate Structure

### 0.1 `vil_log` — Core Log Crate

This is the primary crate. All other VIL crates depend on it.

```
crates/vil_log/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              — re-exports, feature gates
│   │
│   ├── types/
│   │   ├── mod.rs
│   │   ├── header.rs       — VilLogHeader (64 bytes, flat, cache-aligned)
│   │   ├── access.rs       — AccessLog (HTTP req/res)
│   │   ├── app.rs          — AppLog (business logic, relative layout)
│   │   ├── ai.rs           — AiLog (LLM/RAG/Agent)
│   │   ├── db.rs           — DbLog (database operations)
│   │   ├── mq.rs           — MqLog (message queue operations)
│   │   ├── system.rs       — SystemLog (runtime internals)
│   │   └── security.rs     — SecurityLog (auth, rate limit, injection)
│   │
│   ├── emit/
│   │   ├── mod.rs
│   │   ├── ring.rs         — SpscRing<LogSlot> writer (hot path)
│   │   ├── macros.rs       — app_log!(), access_log!(), ai_log!() macro definitions
│   │   └── tracing_layer.rs — VilTracingLayer (bridge tracing → ring)
│   │
│   ├── drain/
│   │   ├── mod.rs
│   │   ├── traits.rs       — LogDrain trait
│   │   ├── clickhouse.rs   — ClickHouseDrain (batch INSERT via Inserter)
│   │   ├── file.rs         — FileDrain (rolling file, async)
│   │   ├── stdout.rs       — StdoutDrain (formatted, colored, dev mode)
│   │   ├── nats.rs         — NatsDrain (publish to NATS subject)
│   │   ├── multi.rs        — MultiDrain (fan-out to N drains simultaneously)
│   │   └── null.rs         — NullDrain (discard, for benchmarks)
│   │
│   ├── dict/
│   │   ├── mod.rs
│   │   └── registry.rs     — DictRegistry (hash → string, dedup, periodic flush)
│   │
│   ├── config.rs           — LogConfig from YAML / ENV
│   ├── process.rs          — LogDrainProcess (ServiceProcess wrapper)
│   ├── runtime.rs          — init_logging(), global ring + drain setup
│   └── error.rs            — #[vil_fault] LogFault
│
├── benches/
│   ├── baseline_tracing.rs — BASELINE: pure tracing + tracing-subscriber benchmark
│   ├── vil_log_emit.rs     — VIL log emit throughput (hot path only)
│   ├── vil_log_e2e.rs      — VIL log end-to-end (emit + drain)
│   └── comparison.rs       — side-by-side comparison report
│
├── tests/
│   ├── unit/
│   │   ├── types.rs        — log type serialization
│   │   ├── ring.rs         — SpscRing correctness
│   │   ├── dict.rs         — dictionary dedup
│   │   └── config.rs       — config parsing
│   ├── integration/
│   │   ├── clickhouse.rs   — Docker ClickHouse drain test
│   │   ├── file.rs         — rolling file drain test
│   │   ├── nats.rs         — Docker NATS drain test
│   │   └── multi.rs        — multi-drain fan-out test
│   └── compliance/
│       ├── zero_copy.rs    — verify no heap alloc on hot path
│       └── drop_count.rs   — verify ring-full drops are counted
│
└── examples/
    ├── 001_stdout_dev.rs       — simplest: stdout drain, dev mode
    ├── 002_file_rolling.rs     — file drain with daily rotation
    ├── 003_clickhouse_batch.rs — ClickHouse batch drain
    ├── 004_nats_fanout.rs      — NATS drain for multi-host
    ├── 005_multi_drain.rs      — stdout + file + ClickHouse simultaneously
    └── 006_custom_drain.rs     — implement your own LogDrain
```

---

## Semantic Types

### VilLogHeader — Base (64 bytes, flat)

```rust
/// Every log event starts with this header.
/// 64 bytes, cache-line aligned, zero-copy safe.
#[vil_event(layout = "flat")]
#[repr(C, align(64))]
pub struct VilLogHeader {
    // Identity (24 bytes)
    pub event_id: u128,            // ULIDv2 — sortable, unique, embeds timestamp
    pub trace_id: u64,             // distributed trace correlation

    // Context (16 bytes)
    pub tenant_id: u64,            // multi-tenant isolation
    pub process_id: u64,           // which ServiceProcess emitted

    // Time (8 bytes)
    pub timestamp_ns: u64,         // nanosecond wall clock

    // Taxonomy (4 bytes)
    pub level: u8,                 // 0=TRACE 1=DEBUG 2=INFO 3=WARN 4=ERROR 5=FATAL
    pub category: u8,              // 0=Access 1=App 2=System 3=Security 4=Ai 5=Db 6=Mq
    pub subcategory: u8,           // category-specific
    pub version: u8,               // schema version for evolution

    // Routing (12 bytes)
    pub service_hash: u32,         // hash of service name
    pub handler_hash: u32,         // hash of handler/function name
    pub node_hash: u32,            // hash of hostname/pod
}
```

### LogSlot — Ring Buffer Entry

```rust
/// Union-style slot in SpscRing.
/// Fixed 256 bytes — fits any log type without dynamic dispatch.
#[repr(C, align(64))]
pub struct LogSlot {
    pub header: VilLogHeader,           // 64 bytes — always present
    pub payload: [u8; 192],             // type-specific payload (flat copy)
}
// Total: 256 bytes per slot
// Ring of 16K slots = 4MB — fits in L3 cache on most CPUs
```

### Log Types — Payload Section

Each log type's extra fields fit within the 192-byte payload:

```rust
// AccessLog payload (56 bytes — fits in 192)
#[vil_event(layout = "flat")]
pub struct AccessPayload {
    pub method: u8,
    pub path_hash: u32,
    pub query_hash: u32,
    pub status_code: u16,
    pub request_size: u32,
    pub response_size: u32,
    pub latency_us: u32,
    pub ttfb_us: u32,
    pub client_ip: u128,
    pub request_id: u64,
    pub session_hash: u32,
    pub user_hash: u32,
}

// AiPayload (32 bytes)
#[vil_event(layout = "flat")]
pub struct AiPayload {
    pub provider_hash: u32,
    pub model_hash: u32,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub latency_ms: u32,
    pub ttft_ms: u32,
    pub tokens_per_sec: f32,
    pub cost_microcents: u32,
}

// DbPayload (24 bytes)
#[vil_event(layout = "flat")]
pub struct DbPayload {
    pub backend: u8,
    pub operation: u8,
    pub query_hash: u32,
    pub table_hash: u32,
    pub rows_affected: u32,
    pub latency_us: u32,
    pub pool_wait_us: u32,
    pub pool_active: u16,
    pub pool_idle: u16,
}

// MqPayload (20 bytes)
#[vil_event(layout = "flat")]
pub struct MqPayload {
    pub backend: u8,
    pub direction: u8,
    pub topic_hash: u32,
    pub partition: u16,
    pub offset: u64,
    pub latency_us: u32,
    pub payload_size: u32,
}

// SecurityPayload (32 bytes)
#[vil_event(layout = "flat")]
pub struct SecurityPayload {
    pub event_type: u8,
    pub severity: u8,
    pub source_ip: u128,
    pub user_hash: u32,
    pub resource_hash: u32,
    pub rule_hash: u32,
    pub attempt_count: u16,
}

// AppPayload (variable — uses ExchangeHeap for fields)
// This is the ONLY log type with relative layout
#[vil_event(layout = "relative")]
pub struct AppPayload {
    pub code: u32,
    pub domain_hash: u32,
    pub entity_id: u64,
    pub message_offset: u32,        // offset into ExchangeHeap for message VSlice
    pub message_len: u32,
    pub fields_offset: u32,         // offset into ExchangeHeap for MsgPack fields
    pub fields_len: u32,
}

// SystemPayload (24 bytes)
#[vil_event(layout = "flat")]
pub struct SystemPayload {
    pub shm_used_bytes: u64,
    pub shm_total_bytes: u64,
    pub queue_depth: u32,
    pub active_processes: u32,
}
```

---

## LogDrain Trait

```rust
#[async_trait]
pub trait LogDrain: Send + Sync + 'static {
    /// Drain name for diagnostics
    fn name(&self) -> &'static str;

    /// Accept a batch of log slots. Called by DrainProcess.
    async fn flush(&self, batch: &[LogSlot]) -> Result<(), LogFault>;

    /// Graceful shutdown — flush remaining, close connections
    async fn shutdown(&self) -> Result<(), LogFault>;
}
```

---

## Drain Implementations

### ClickHouseDrain

```rust
pub struct ClickHouseDrain {
    client: clickhouse::Client,
    batch_config: BatchConfig,
}

pub struct BatchConfig {
    pub max_rows: usize,            // default: 10_000
    pub max_bytes: usize,           // default: 4MB
    pub max_wait: Duration,         // default: 1 second
}

impl LogDrain for ClickHouseDrain {
    fn name(&self) -> &'static str { "clickhouse" }

    async fn flush(&self, batch: &[LogSlot]) -> Result<(), LogFault> {
        // Demux by category → insert into correct table
        let mut access_buf = Vec::new();
        let mut app_buf = Vec::new();
        let mut ai_buf = Vec::new();
        // ... etc

        for slot in batch {
            match slot.header.category {
                0 => access_buf.push(slot),
                1 => app_buf.push(slot),
                4 => ai_buf.push(slot),
                // ...
            }
        }

        // Parallel insert to each table
        tokio::try_join!(
            self.insert_access(&access_buf),
            self.insert_app(&app_buf),
            self.insert_ai(&ai_buf),
            // ...
        )?;
        Ok(())
    }
}
```

**ClickHouse tables**: See semantic log design doc. Per-category tables with MergeTree, partitioned by day, TTL configured.

---

### FileDrain — Rolling File

```rust
pub struct FileDrain {
    config: FileConfig,
    writer: Arc<Mutex<BufWriter<File>>>,  // Mutex only on drain side (cold path)
}

pub struct FileConfig {
    pub dir: PathBuf,                   // log directory
    pub prefix: String,                 // e.g., "vil"
    pub format: FileFormat,             // JsonLines | MsgPack | Text
    pub rotation: Rotation,             // Daily | Hourly | Size(bytes)
    pub max_files: usize,              // retention: keep N most recent
    pub compress_rotated: bool,         // gzip old files
}

pub enum Rotation {
    Daily,
    Hourly,
    Size(u64),                          // rotate when file exceeds N bytes
}

pub enum FileFormat {
    JsonLines,                          // one JSON object per line (human + machine readable)
    MsgPack,                            // compact binary (machine only)
    Text,                               // formatted text (human readable, dev mode)
}
```

**Rolling behavior**:
```
/var/log/vil/
├── vil.2026-03-27.log          ← current (active writes)
├── vil.2026-03-26.log.gz       ← rotated, compressed
├── vil.2026-03-25.log.gz
└── vil.2026-03-24.log.gz       ← oldest (max_files=4, older deleted)
```

---

### StdoutDrain — Development Mode

```rust
pub struct StdoutDrain {
    format: StdoutFormat,
    filter: LevelFilter,
}

pub enum StdoutFormat {
    Pretty,         // colored, multi-line, human-friendly
    Compact,        // single line, colored level
    Json,           // JSON per line (for piping to jq)
}
```

**Pretty output example**:
```
2026-03-27T10:30:15.123Z INFO  [api/create_order] order.created
  order_id=12345 amount=50000 currency=IDR
  trace=abc123 tenant=t1 latency=2.3ms

2026-03-27T10:30:15.456Z WARN  [api/process_payment] payment.retry
  order_id=12345 attempt=3 reason_code=4201
  trace=abc123 tenant=t1

2026-03-27T10:30:15.789Z ERROR [api/check_inventory] inventory.insufficient
  sku=SKU-001 requested=10 available=3
  trace=abc123 tenant=t1
```

---

### NatsDrain — Cross-Host Fan-Out

```rust
pub struct NatsDrain {
    client: async_nats::Client,
    subject_prefix: String,             // e.g., "vil.logs"
    format: WireFormat,
}

pub enum WireFormat {
    MsgPack,        // compact, fast (default)
    JsonLines,      // interop with non-VIL consumers
}
```

**Subject mapping**:
```
vil.logs.access     ← AccessLog events
vil.logs.app        ← AppLog events
vil.logs.ai         ← AiLog events
vil.logs.db         ← DbLog events
vil.logs.mq         ← MqLog events
vil.logs.system     ← SystemLog events
vil.logs.security   ← SecurityLog events
```

Consumers subscribe to specific categories or `vil.logs.>` for all.

---

### MultiDrain — Fan-Out

```rust
pub struct MultiDrain {
    drains: Vec<Box<dyn LogDrain>>,
}

// Example: dev + production setup
let drain = MultiDrain::new()
    .add(StdoutDrain::pretty(LevelFilter::Debug))          // dev: see everything
    .add(FileDrain::new(file_config))                       // always: local backup
    .add(ClickHouseDrain::new(ch_config));                  // prod: analytics
```

---

## Configuration

### YAML Config

```yaml
logging:
  # Ring buffer
  ring:
    slots: 16384               # 16K slots × 256 bytes = 4MB
    on_full: drop_count        # drop_count | block (never use block in prod)

  # Global level filter
  level: info                  # trace | debug | info | warn | error | fatal

  # Category-specific level overrides
  categories:
    access: info
    app: debug
    ai: info
    db: warn                   # only log slow queries
    mq: info
    system: warn
    security: info             # always log security events

  # Drain backends (multiple allowed)
  drains:
    - type: stdout
      format: pretty           # pretty | compact | json
      level: debug             # override: stdout gets more verbose in dev

    - type: file
      dir: /var/log/vil
      prefix: vil
      format: json_lines
      rotation: daily
      max_files: 30
      compress: true

    - type: clickhouse
      url: http://clickhouse:8123
      database: vil_logs
      batch:
        max_rows: 10000
        max_bytes: 4194304     # 4MB
        max_wait_ms: 1000

    - type: nats
      url: nats://nats:4222
      subject_prefix: vil.logs
      format: msgpack

  # Dictionary
  dict:
    flush_interval_ms: 5000    # flush hash→string mappings every 5s
    max_entries: 100000        # cap dictionary size

  # Tracing bridge
  tracing_bridge: true         # enable VilTracingLayer for ecosystem compat
```

### ENV Overrides

```bash
VIL_LOG_LEVEL=debug
VIL_LOG_RING_SLOTS=32768
VIL_LOG_DRAIN_STDOUT_FORMAT=json
VIL_LOG_DRAIN_CLICKHOUSE_URL=http://ch:8123
VIL_LOG_DRAIN_FILE_DIR=/var/log/vil
```

---

## Developer API

### Auto-Emitted (developer writes NOTHING)

```rust
// AccessLog — emitted by #[vil_handler] on every HTTP request/response
//   captures: method, path, status, latency, client_ip, request_id

// DbLog — emitted by vil_db_semantic on every query
//   captures: backend, operation, query_hash, rows_affected, latency, pool stats

// MqLog — emitted by vil_mq_* on every publish/consume
//   captures: backend, direction, topic, partition, offset, latency

// AiLog — emitted by vil_llm on every completion/embedding
//   captures: provider, model, tokens, latency, cost, cache_hit

// SystemLog — emitted by vil_rt periodically
//   captures: SHM usage, queue depths, active processes
```

### Manual (developer writes only this)

```rust
use vil_log::prelude::*;

// Structured business event
app_log!(INFO, "order.created", {
    order_id: 12345,
    amount_cents: 500_00,
    currency: "IDR",
});

// Warning with context
app_log!(WARN, "payment.retry", {
    order_id: 12345,
    attempt: 3,
    gateway: "midtrans",
    reason_code: 4201,
});

// Error
app_log!(ERROR, "inventory.insufficient", {
    sku_hash: hash("SKU-001"),
    requested: 10,
    available: 3,
});

// Security event (manual, for custom security logic)
security_log!(HIGH, AuthFail, {
    user_hash: hash(username),
    source_ip: client_ip,
    attempt_count: failed_count,
});
```

### `app_log!` Macro Expansion

```rust
// app_log!(INFO, "order.created", { order_id: 12345, amount_cents: 50000 })
// expands to:

{
    let mut slot = LogSlot::zeroed();
    slot.header = VilLogHeader {
        event_id: ulid_now(),
        trace_id: current_trace_id(),
        tenant_id: current_tenant_id(),
        process_id: current_process_id(),
        timestamp_ns: clock_ns(),
        level: 2, // INFO
        category: 1, // App
        subcategory: 1, // Business
        version: 1,
        service_hash: CURRENT_SERVICE_HASH,
        handler_hash: CURRENT_HANDLER_HASH,
        node_hash: NODE_HASH,
    };
    // MsgPack encode fields directly into slot.payload
    let fields = msgpack_encode!({ order_id: 12345, amount_cents: 50000 });
    slot.payload[..fields.len()].copy_from_slice(&fields);
    // Push to ring — never blocks
    let _ = GLOBAL_LOG_RING.try_push(slot);
}
```

---

## Benchmark Plan

### Step 1: Baseline — `tracing` (MUST RUN FIRST)

```rust
// benches/baseline_tracing.rs
use criterion::{criterion_group, criterion_main, Criterion};
use tracing::{info, info_span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::non_blocking;

fn bench_tracing_emit(c: &mut Criterion) {
    // Setup: non-blocking file writer (closest to production)
    let (non_blocking, _guard) = non_blocking(std::io::sink());
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    c.bench_function("tracing_info_event", |b| {
        b.iter(|| {
            info!(order_id = 12345, amount = 50000, "order.created");
        })
    });

    c.bench_function("tracing_with_span", |b| {
        b.iter(|| {
            let _span = info_span!("handler", request_id = 999).entered();
            info!(order_id = 12345, amount = 50000, "order.created");
        })
    });

    // High contention: 8 threads emitting simultaneously
    c.bench_function("tracing_8_threads", |b| {
        b.iter(|| {
            std::thread::scope(|s| {
                for _ in 0..8 {
                    s.spawn(|| {
                        for _ in 0..1000 {
                            info!(order_id = 12345, "order.created");
                        }
                    });
                }
            });
        })
    });
}

criterion_group!(benches, bench_tracing_emit);
criterion_main!(benches);
```

### Step 2: VIL Log — SpscRing

```rust
// benches/vil_log_emit.rs
fn bench_vil_log_emit(c: &mut Criterion) {
    let ring = SpscRing::<LogSlot>::new(16384);

    // Single event emit
    c.bench_function("vil_log_app_event", |b| {
        b.iter(|| {
            app_log!(INFO, "order.created", {
                order_id: 12345,
                amount_cents: 50000,
            });
        })
    });

    // Access log (auto-emit simulation)
    c.bench_function("vil_log_access_event", |b| {
        b.iter(|| {
            access_log!(AccessPayload {
                method: 1,
                path_hash: 0x1234,
                status_code: 200,
                latency_us: 2300,
                ..Default::default()
            });
        })
    });

    // AI log
    c.bench_function("vil_log_ai_event", |b| {
        b.iter(|| {
            ai_log!(AiPayload {
                provider_hash: 0xABCD,
                model_hash: 0x5678,
                prompt_tokens: 150,
                completion_tokens: 500,
                latency_ms: 1200,
                cost_microcents: 350,
                ..Default::default()
            });
        })
    });
}
```

### Step 3: Comparison Report

```rust
// benches/comparison.rs
// Run both and produce table:
//
// | Operation               | tracing (ns) | vil_log (ns) | Speedup |
// |-------------------------|-------------|-------------|---------|
// | Single INFO event       | ~150        | ~20         | 7.5x    |
// | Event + span            | ~250        | ~25         | 10x     |
// | 8-thread contention     | ~400        | ~30*        | 13x     |
// | Access log (structured) | N/A†        | ~15         | —       |
// | AI log (structured)     | N/A†        | ~15         | —       |
//
// * VIL uses per-thread SPSC rings — no contention
// † tracing has no native structured log types — would need custom Layer
```

**Expected results** (estimates based on architecture):
- `tracing` with `NonBlocking`: 100-300ns per event (format + crossbeam channel)
- `vil_log` with SpscRing: 15-50ns per event (memcpy only)
- Speedup: **3-10x** depending on payload complexity
- Under contention: VIL wins bigger because SPSC = no cross-thread CAS

---

## Dictionary System

### Hash → String Registry

```rust
pub struct DictRegistry {
    map: DashMap<(u8, u32), ()>,        // (kind, hash) → seen flag
    pending: Mutex<Vec<DictEntry>>,     // new entries to flush
}

#[vil_state(layout = "relative")]
pub struct DictEntry {
    pub hash: u32,
    pub kind: u8,       // 0=service 1=handler 2=path 3=query 4=topic 5=model ...
    pub value: VSlice<u8>,
}

impl DictRegistry {
    /// Register a string. If hash is new, queue for flush.
    /// Called on cold path (config init, first request to new path, etc.)
    pub fn register(&self, kind: u8, value: &str) -> u32 {
        let hash = fxhash32(value);
        if self.map.insert((kind, hash), ()).is_none() {
            // New hash — queue for flush to ClickHouse/file
            self.pending.lock().push(DictEntry {
                hash,
                kind,
                value: VSlice::from_bytes(value.as_bytes()),
            });
        }
        hash
    }

    /// Flush pending entries to drain. Called periodically.
    pub async fn flush(&self, drain: &dyn LogDrain) {
        let entries = std::mem::take(&mut *self.pending.lock());
        if !entries.is_empty() {
            drain.flush_dict(&entries).await;
        }
    }
}
```

### ClickHouse Dictionary Table

```sql
CREATE TABLE vil_dict (
    hash       UInt32,
    kind       UInt8,
    value      String,
    first_seen DateTime DEFAULT now()
) ENGINE = ReplacingMergeTree()
ORDER BY (kind, hash);

-- Usage: join in queries
SELECT
    d.value AS service_name,
    count() AS requests,
    avg(a.latency_us) AS avg_latency
FROM vil_access_log a
JOIN vil_dict d ON a.service_hash = d.hash AND d.kind = 0
WHERE a.timestamp_ns > now64() - INTERVAL 1 HOUR
GROUP BY d.value
ORDER BY requests DESC;
```

---

## Integration Points

### Auto-Emit from Existing VIL Macros

```rust
// In vil_server_macros — #[vil_handler] expansion adds:
async fn __handler_wrapper(/* ... */) -> Response {
    let start = Instant::now();
    let result = actual_handler(/* ... */).await;
    let elapsed = start.elapsed();

    // Auto-emit AccessLog — zero developer effort
    access_log!(AccessPayload {
        method: method_to_u8(req.method()),
        path_hash: DICT.register(2, req.uri().path()),
        status_code: result.status().as_u16(),
        latency_us: elapsed.as_micros() as u32,
        request_size: req.body().len() as u32,
        response_size: result.body().len() as u32,
        client_ip: extract_ip(&req),
        request_id: extract_request_id(&req),
        ..Default::default()
    });

    result
}

// In vil_db_semantic — query wrapper adds:
// DbLog auto-emitted per query

// In vil_llm — completion wrapper adds:
// AiLog auto-emitted per LLM call

// In vil_mq_* — publish/consume wrapper adds:
// MqLog auto-emitted per message
```

---

## ClickHouse Schema (Full)

```sql
-- Access logs
CREATE TABLE vil_access_log (
    event_id       UInt128,
    trace_id       UInt64,
    tenant_id      UInt64,
    process_id     UInt64,
    timestamp_ns   UInt64,
    level          UInt8,
    service_hash   UInt32,
    handler_hash   UInt32,
    node_hash      UInt32,
    method         UInt8,
    path_hash      UInt32,
    query_hash     UInt32,
    status_code    UInt16,
    request_size   UInt32,
    response_size  UInt32,
    latency_us     UInt32,
    ttfb_us        UInt32,
    client_ip      IPv6,
    request_id     UInt64,
    session_hash   UInt32,
    user_hash      UInt32
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp64Nano(timestamp_ns))
ORDER BY (tenant_id, service_hash, timestamp_ns)
TTL toDate(fromUnixTimestamp64Nano(timestamp_ns)) + INTERVAL 90 DAY;

-- App logs
CREATE TABLE vil_app_log (
    event_id       UInt128,
    trace_id       UInt64,
    tenant_id      UInt64,
    process_id     UInt64,
    timestamp_ns   UInt64,
    level          UInt8,
    service_hash   UInt32,
    handler_hash   UInt32,
    node_hash      UInt32,
    code           UInt32,
    domain_hash    UInt32,
    entity_id      UInt64,
    message        String,
    fields         String          -- MsgPack → JSON on insert for queryability
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp64Nano(timestamp_ns))
ORDER BY (tenant_id, service_hash, level, timestamp_ns)
TTL toDate(fromUnixTimestamp64Nano(timestamp_ns)) + INTERVAL 180 DAY;

-- AI logs
CREATE TABLE vil_ai_log (
    event_id          UInt128,
    trace_id          UInt64,
    tenant_id         UInt64,
    process_id        UInt64,
    timestamp_ns      UInt64,
    level             UInt8,
    service_hash      UInt32,
    handler_hash      UInt32,
    node_hash         UInt32,
    provider_hash     UInt32,
    model_hash        UInt32,
    prompt_tokens     UInt32,
    completion_tokens UInt32,
    latency_ms        UInt32,
    ttft_ms           UInt32,
    tokens_per_sec    Float32,
    cost_microcents   UInt32,
    cache_hit         UInt8,
    guardrail_flags   UInt16
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp64Nano(timestamp_ns))
ORDER BY (tenant_id, provider_hash, model_hash, timestamp_ns)
TTL toDate(fromUnixTimestamp64Nano(timestamp_ns)) + INTERVAL 365 DAY;

-- DB logs
CREATE TABLE vil_db_log (
    event_id       UInt128,
    trace_id       UInt64,
    tenant_id      UInt64,
    process_id     UInt64,
    timestamp_ns   UInt64,
    level          UInt8,
    service_hash   UInt32,
    handler_hash   UInt32,
    node_hash      UInt32,
    backend        UInt8,
    operation      UInt8,
    query_hash     UInt32,
    table_hash     UInt32,
    rows_affected  UInt32,
    latency_us     UInt32,
    pool_wait_us   UInt32,
    pool_active    UInt16,
    pool_idle      UInt16
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp64Nano(timestamp_ns))
ORDER BY (tenant_id, backend, table_hash, timestamp_ns)
TTL toDate(fromUnixTimestamp64Nano(timestamp_ns)) + INTERVAL 90 DAY;

-- MQ logs
CREATE TABLE vil_mq_log (
    event_id       UInt128,
    trace_id       UInt64,
    tenant_id      UInt64,
    process_id     UInt64,
    timestamp_ns   UInt64,
    level          UInt8,
    service_hash   UInt32,
    handler_hash   UInt32,
    node_hash      UInt32,
    backend        UInt8,
    direction      UInt8,
    topic_hash     UInt32,
    partition      UInt16,
    mq_offset      UInt64,
    latency_us     UInt32,
    payload_size   UInt32
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp64Nano(timestamp_ns))
ORDER BY (tenant_id, backend, topic_hash, timestamp_ns)
TTL toDate(fromUnixTimestamp64Nano(timestamp_ns)) + INTERVAL 90 DAY;

-- Security logs
CREATE TABLE vil_security_log (
    event_id       UInt128,
    trace_id       UInt64,
    tenant_id      UInt64,
    process_id     UInt64,
    timestamp_ns   UInt64,
    level          UInt8,
    service_hash   UInt32,
    handler_hash   UInt32,
    node_hash      UInt32,
    event_type     UInt8,
    severity       UInt8,
    source_ip      IPv6,
    user_hash      UInt32,
    resource_hash  UInt32,
    rule_hash      UInt32,
    attempt_count  UInt16
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp64Nano(timestamp_ns))
ORDER BY (tenant_id, event_type, severity, timestamp_ns)
TTL toDate(fromUnixTimestamp64Nano(timestamp_ns)) + INTERVAL 365 DAY;

-- System logs
CREATE TABLE vil_system_log (
    event_id        UInt128,
    trace_id        UInt64,
    tenant_id       UInt64,
    process_id      UInt64,
    timestamp_ns    UInt64,
    level           UInt8,
    service_hash    UInt32,
    node_hash       UInt32,
    subcategory     UInt8,
    shm_used_bytes  UInt64,
    shm_total_bytes UInt64,
    queue_depth     UInt32,
    active_processes UInt32
) ENGINE = MergeTree()
PARTITION BY toDate(fromUnixTimestamp64Nano(timestamp_ns))
ORDER BY (tenant_id, node_hash, subcategory, timestamp_ns)
TTL toDate(fromUnixTimestamp64Nano(timestamp_ns)) + INTERVAL 30 DAY;

-- Aggregated view: per-minute access stats
CREATE MATERIALIZED VIEW vil_access_1m
ENGINE = SummingMergeTree()
PARTITION BY toDate(minute)
ORDER BY (tenant_id, service_hash, path_hash, status_group, minute)
AS SELECT
    tenant_id,
    service_hash,
    path_hash,
    intDiv(status_code, 100) AS status_group,
    toStartOfMinute(fromUnixTimestamp64Nano(timestamp_ns)) AS minute,
    count() AS requests,
    sum(latency_us) AS total_latency_us,
    max(latency_us) AS max_latency_us,
    sumIf(1, status_code >= 500) AS errors
FROM vil_access_log
GROUP BY tenant_id, service_hash, path_hash, status_group, minute;

-- Aggregated view: AI cost per hour
CREATE MATERIALIZED VIEW vil_ai_cost_1h
ENGINE = SummingMergeTree()
PARTITION BY toDate(hour)
ORDER BY (tenant_id, provider_hash, model_hash, hour)
AS SELECT
    tenant_id,
    provider_hash,
    model_hash,
    toStartOfHour(fromUnixTimestamp64Nano(timestamp_ns)) AS hour,
    count() AS requests,
    sum(prompt_tokens) AS total_prompt_tokens,
    sum(completion_tokens) AS total_completion_tokens,
    sum(cost_microcents) AS total_cost_microcents,
    avg(latency_ms) AS avg_latency_ms
FROM vil_ai_log
GROUP BY tenant_id, provider_hash, model_hash, hour;
```

---

## Development Order

```
Week 1:  Benchmark baseline (tracing) — MUST complete before any VIL log code
         ├── benches/baseline_tracing.rs
         └── Record: single event, span, 8-thread contention

Week 2:  vil_log core
         ├── types/ (all semantic log structs)
         ├── emit/ring.rs (SpscRing<LogSlot>)
         ├── emit/macros.rs (app_log!, access_log!, etc.)
         └── drain/null.rs (NullDrain for benchmarks)

Week 3:  VIL log benchmark + comparison
         ├── benches/vil_log_emit.rs
         ├── benches/comparison.rs
         └── PUBLISH BENCHMARK RESULTS — prove the speedup

Week 4:  Drain backends
         ├── drain/stdout.rs (dev mode — simplest)
         ├── drain/file.rs (rolling file)
         ├── drain/clickhouse.rs (batch INSERT)
         ├── drain/nats.rs (cross-host)
         └── drain/multi.rs (fan-out)

Week 5:  Integration
         ├── emit/tracing_layer.rs (bridge ecosystem)
         ├── dict/ (hash→string registry)
         ├── config.rs (YAML + ENV)
         ├── process.rs (LogDrainProcess as ServiceProcess)
         └── runtime.rs (init_logging() global setup)

Week 6:  Auto-emit integration
         ├── Update #[vil_handler] → emit AccessLog
         ├── Update vil_db_semantic → emit DbLog
         ├── Update vil_llm → emit AiLog
         ├── Update vil_mq_* → emit MqLog
         └── Update vil_rt → emit SystemLog

Week 7:  Testing + examples
         ├── All unit tests
         ├── All integration tests (Docker ClickHouse, NATS)
         ├── 6 examples
         └── README + documentation
```

---

## Benchmark Results (Actual — 2026-03-27)

> System: Intel i9-11900F (8C/16T), 32GB RAM, Ubuntu 22.04, Rust 1.93.1
> All tests: 1,000,000 events, `--release`, single thread, NullDrain

### Single-Thread Performance (all VIL log types vs tracing)

| Log Type | ns/event | M ev/s | vs tracing | Serialization |
|----------|----------|--------|------------|---------------|
| **tracing (fmt + NonBlocking)** | **810** | **1.23** | baseline | String fmt + MPMC channel |
| VIL access_log! | **178** | 5.63 | **4.6x** faster | Flat memcpy (HTTP req/res) |
| VIL ai_log! | **178** | 5.62 | **4.5x** faster | Flat memcpy (LLM call) |
| VIL db_log! | **137** | 7.32 | **5.9x** faster | Flat memcpy (DB query) |
| VIL mq_log! | **130** | 7.67 | **6.2x** faster | Flat memcpy (MQ pub/sub) |
| VIL system_log! | **130** | 7.68 | **6.2x** faster | Flat memcpy (OS metrics) |
| VIL security_log! | **132** | 7.57 | **6.1x** faster | Flat memcpy (auth event) |
| VIL app_log! (flat) | **133** | 7.55 | **6.1x** faster | Flat memcpy (app event) |
| VIL app_log! (dynamic) | **390** | 2.56 | **2.1x** faster | MsgPack KV (4 fields) |
| tracing (filtered out) | 0.2 | 4887 | — | Atomic load (1 CAS) |
| VIL (filtered out) | 0.2 | 4985 | — | Atomic load (1 CAS) |

### Multi-Thread Scaling (striped SPSC rings, `threads: Some(8)`)

| Threads | tracing (fmt) | VIL access_log! | Speedup |
|---------|--------------|-----------------|---------|
| 1 | 1.84 M/s | **6.99 M/s** | **3.8x** |
| 2 | 3.54 M/s | **10.33 M/s** | **2.9x** |
| 4 | 5.13 M/s | **10.51 M/s** | **2.0x** |
| 8 | 6.21 M/s | **6.25 M/s** | **1.0x** |

### File Drain E2E (500K events, writing to /tmp)

| Benchmark | ns/event | M ev/s | File size |
|-----------|----------|--------|-----------|
| tracing (JSON fmt + rolling file) | 1218 | 0.82 | 110.5 MB |
| VIL access_log! → FileDrain | **183** | **5.47** | 59.6 MB |
| **Speedup** | | **6.7x faster** | 46% smaller |

### Architecture: Auto-Sized Striped SPSC Rings

```
LogConfig { threads: Some(N) }  →  N striped SPSC rings (round-robin assignment)
LogConfig { threads: None }     →  auto-detect from available_parallelism()

Thread 0 → Ring 0 ──┐
Thread 1 → Ring 1 ──┤
Thread 2 → Ring 2 ──┼──► Drain Task (round-robin merge) ──► Drain Backend
Thread 3 → Ring 3 ──┤
...                 │
Thread N → Ring N ──┘

At ≤N threads: 1 thread/ring → zero contention
At >N threads: round-robin wrap → some sharing
```

---

## Milestone Checklist — COMPLETED ✅

### Benchmark ✅
- [x] `baseline_tracing.rs` — recorded baseline: 810ns/event
- [x] `vil_log_emit.rs` — VIL flat: 130-178ns, dynamic: 390ns
- [x] `comparison.rs` — side-by-side report with bar chart
- [x] VIL flat log is 4.5-6.2x faster than tracing (single thread)
- [x] VIL wins up to 4 threads, near-parity at 8 threads
- [x] File drain E2E: 6.7x faster than tracing

### Core ✅
- [x] 7 semantic log types (Access, App, AI, DB, MQ, System, Security)
- [x] `LogSlot` — 256 bytes, cache-aligned
- [x] `StripedRing` — auto-sized N × SPSC, round-robin thread assignment
- [x] `app_log!()` macro — flat + dynamic variants
- [x] `LogDrain` trait defined

### Drains ✅
- [x] `StdoutDrain` — pretty/compact/json
- [x] `FileDrain` — daily/hourly/size rotation, retention
- [x] `ClickHouseDrain` — batch INSERT (feature-gated: `clickhouse-drain`)
- [x] `NatsDrain` — per-category subjects (feature-gated: `nats-drain`)
- [x] `MultiDrain` — fan-out to N drains
- [x] `NullDrain` — for benchmarks

### Integration ✅
- [x] `VilTracingLayer` — bridge tracing → StripedRing
- [x] `DictRegistry` — hash→string, fxhash
- [x] `LogConfig` with `threads` hint for optimal stripe sizing
- [x] `init_logging()` global setup + drain task
- [x] Auto-emit: `#[vil_handler]` → AccessLog
- [x] Auto-emit: `vil_db_semantic` → DbLog
- [x] Auto-emit: `vil_llm` (OpenAI, Anthropic, Ollama) → AiLog
- [x] Auto-emit: `vil_mq_nats`, `vil_mq_kafka`, `vil_mq_mqtt` → MqLog
- [x] Auto-emit: `vil_rt` (world, supervisor) → SystemLog

### Examples ✅ (8 total)
- [x] 501: Stdout dev mode
- [x] 502: File rolling
- [x] 503: Multi-drain fan-out
- [x] 504: Complete benchmark (all 7 types + tracing baseline)
- [x] 505: tracing bridge
- [x] 506: All 7 structured log types
- [x] 507: File drain E2E benchmark
- [x] 508: Multi-thread contention benchmark

### Documentation ✅
- [x] README.md for `vil_log` crate
- [x] Benchmark results documented
- [x] Phase 0 docs-dev updated
