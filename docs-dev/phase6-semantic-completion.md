# Phase 6 — Semantic Completion: Full VIL Way for All Crates

> **⚠ MANDATORY: Read [COMPLIANCE.md](./COMPLIANCE.md) before implementing.**
> This phase retrofits full VIL semantic compliance onto all Phase 1-5 crates.

## Problem Statement

Phase 1-5 crates (30 connectors) have:
- ✅ `vil_log` integration (db_log!, mq_log!)
- ✅ Plain enum faults (u32 fields, no heap)
- ✅ `process.rs` (ServiceProcess pattern)
- ❌ `#[vil_fault]` derive macro (requires heavy dep chain)
- ❌ `#[vil_event]` / `#[vil_state]` semantic types
- ❌ YAML codegen for connectors
- ❌ SDK templates for connectors

## Solution: 4 Work Streams

---

### Stream 1: `vil_connector_macros` — Lightweight Semantic Derive

**Problem:** `#[vil_fault]` from `vil_macros` requires `vil_sdk` + `vil_ir` + `vil_validate` = 5 crate dep chain. Too heavy for connector crates.

**Solution:** New lightweight proc-macro crate that provides connector-specific derives WITHOUT the full IR/validation pipeline.

```
crates/vil_connector_macros/
├── Cargo.toml        # proc-macro, depends ONLY on syn + quote + proc-macro2
├── src/
│   └── lib.rs
```

**What it provides:**

```rust
// #[connector_fault] — lightweight version of #[vil_fault]
// Generates: Display, From conversions, error_code() method, FaultKind enum
#[connector_fault]
pub enum MongoFault {
    ConnectionFailed { uri_hash: u32, reason_code: u32 },
    QueryFailed { collection_hash: u32, reason_code: u32 },
    Timeout { collection_hash: u32, elapsed_ms: u32 },
}

// Expands to:
impl MongoFault {
    pub fn error_code(&self) -> u32 { ... }
    pub fn kind(&self) -> &'static str { ... }  // "ConnectionFailed", "QueryFailed", etc.
    pub fn is_retryable(&self) -> bool { ... }   // Timeout = true, others = false
}
impl std::fmt::Display for MongoFault { ... }
impl std::error::Error for MongoFault { ... }

// #[connector_event] — defines an event emitted by the connector
#[connector_event]
pub struct MongoChangeEvent {
    pub collection_hash: u32,
    pub operation: u8,      // 0=insert, 1=update, 2=delete
    pub document_id_hash: u32,
    pub timestamp_ns: u64,
}

// Expands to: Debug, Clone, Copy, Default, #[repr(C)] validation

// #[connector_state] — defines connector state for ServiceProcess
#[connector_state]
pub struct MongoPoolState {
    pub active_connections: u32,
    pub idle_connections: u32,
    pub waiting_requests: u32,
    pub total_queries: u64,
}
```

**Dependency chain:** `vil_connector_macros` depends ONLY on `syn`, `quote`, `proc-macro2`. Zero VIL runtime deps. ~2 second compile.

**Migration:** Each connector crate:
1. Add `vil_connector_macros = { workspace = true }` to Cargo.toml
2. Replace plain `pub enum XxxFault { ... }` with `#[connector_fault] pub enum XxxFault { ... }`
3. Remove hand-written `impl Display`, `impl Default`, `as_error_code()` — macro generates these
4. All 28 crates must be migrated — no plain enum faults remaining

**Consistency rule:** After this stream, EVERY fault enum in the project uses either:
- `#[vil_fault]` (for crates that depend on `vil_macros` — core, server, SDK)
- `#[connector_fault]` (for connector/trigger crates — lightweight)

Plain enum without derive is **prohibited** going forward.

**Estimated effort:** 3-4 days

---

### Stream 2: Connector Events & State Types

**Problem:** Connector crates don't define `#[vil_event]` or `#[vil_state]` — they just do operations and log. No structured events flow on Tri-Lane.

**Solution:** Each connector crate gets `events.rs` and `state.rs` modules.

**Design Pattern:**

```rust
// events.rs — what the connector can emit on Data Lane
#[connector_event]
pub struct S3ObjectCreated {
    pub bucket_hash: u32,
    pub key_hash: u32,
    pub size_bytes: u64,
    pub etag_hash: u32,
    pub timestamp_ns: u64,
}

#[connector_event]
pub struct S3ObjectDeleted {
    pub bucket_hash: u32,
    pub key_hash: u32,
    pub timestamp_ns: u64,
}

// state.rs — connector health/metrics for ServiceProcess
#[connector_state]
pub struct S3ClientState {
    pub total_puts: u64,
    pub total_gets: u64,
    pub total_errors: u64,
    pub avg_latency_us: u32,
}
```

**Per-crate event/state definitions:**

| Crate | Events | State |
|-------|--------|-------|
| **vil_storage_s3** | ObjectCreated, ObjectDeleted | S3ClientState (puts, gets, errors, latency) |
| **vil_storage_gcs** | ObjectCreated, ObjectDeleted | GcsClientState |
| **vil_storage_azure** | BlobCreated, BlobDeleted | AzureClientState |
| **vil_db_mongo** | DocumentInserted, DocumentUpdated, DocumentDeleted | MongoPoolState (active, idle, queries) |
| **vil_db_clickhouse** | BatchInserted | ChClientState (inserts, queries, batch_size) |
| **vil_db_dynamodb** | ItemPut, ItemDeleted | DynamoClientState |
| **vil_db_cassandra** | QueryExecuted | CassandraPoolState |
| **vil_db_timeseries** | PointsWritten | TimeseriesClientState |
| **vil_db_neo4j** | NodeCreated, RelationCreated | Neo4jSessionState |
| **vil_db_elastic** | DocumentIndexed, SearchExecuted | ElasticClientState |
| **vil_mq_rabbitmq** | MessagePublished, MessageConsumed, MessageAcked | RabbitChannelState |
| **vil_mq_sqs** | MessageSent, MessageReceived, MessageDeleted | SqsQueueState |
| **vil_mq_pulsar** | MessageSent, MessageReceived | PulsarProducerState |
| **vil_mq_pubsub** | MessagePublished, MessageReceived | PubSubState |
| **vil_soap** | ActionCalled | SoapClientState |
| **vil_opcua** | NodeRead, NodeWritten, ValueSubscribed | OpcUaSessionState |
| **vil_modbus** | RegisterRead, RegisterWritten | ModbusClientState |
| **vil_ws** | MessageSent, MessageReceived, ClientConnected, ClientDisconnected | WsServerState |
| **vil_trigger_core** | TriggerFired (already exists) | TriggerState |
| **vil_trigger_cron** | CronFired | CronTriggerState |
| **vil_trigger_fs** | FileChanged | FsTriggerState |
| **vil_trigger_cdc** | RowChanged | CdcTriggerState |
| **vil_trigger_email** | EmailReceived | EmailTriggerState |
| **vil_trigger_iot** | DeviceEvent | IotTriggerState |
| **vil_trigger_evm** | LogEmitted | EvmTriggerState |
| **vil_trigger_webhook** | WebhookReceived | WebhookTriggerState |
| **vil_otel** | — | OtelExportState (spans_exported, metrics_exported) |
| **vil_edge_deploy** | — | EdgeDeployState |

**All event structs:** `#[repr(C)]`, ≤192 bytes (fits in LogSlot payload), u32 hashes for strings.

**All state structs:** `#[repr(C)]`, atomic counters for live metrics.

**Estimated effort:** 5-7 days (mechanical — follow pattern per crate)

---

### Stream 3: YAML Codegen for Connectors

**Problem:** `vil compile` only knows about HTTP source/sink. Doesn't know about MongoDB, S3, Kafka, triggers, etc.

**Solution:** Extend the YAML manifest schema to support connector declarations.

**New YAML sections:**

```yaml
# app.vil.yaml — extended with connectors
name: my-service
port: 3080

# Existing
pipeline:
  source:
    type: http
    url: http://upstream:4545
    format: sse

# NEW — Connector declarations
connectors:
  databases:
    - name: primary-mongo
      type: mongo
      uri: ${MONGO_URI}
      database: myapp

    - name: analytics
      type: clickhouse
      url: ${CLICKHOUSE_URL}
      database: analytics

  storage:
    - name: uploads
      type: s3
      endpoint: ${S3_ENDPOINT}
      bucket: uploads
      region: us-east-1

  queues:
    - name: events
      type: rabbitmq
      uri: ${RABBITMQ_URI}
      exchange: events
      queue: processing

  triggers:
    - name: daily-report
      type: cron
      schedule: "0 30 6 * * *"

    - name: file-watcher
      type: fs
      path: /data/incoming
      pattern: "*.csv"

# NEW — Logging configuration
logging:
  level: info
  threads: 4
  drains:
    - type: stdout
      format: resolved
    - type: clickhouse
      url: ${CLICKHOUSE_URL}
      database: vil_logs
```

**Codegen changes:**

1. **`crates/vil_cli/src/manifest.rs`** — parse new `connectors`, `triggers`, `logging` YAML sections
2. **`crates/vil_cli/src/codegen.rs`** — generate Rust code for connector init:
   ```rust
   // Auto-generated from YAML
   let mongo = vil_db_mongo::process::create_client(MongoConfig {
       uri: std::env::var("MONGO_URI").unwrap(),
       database: "myapp".into(),
       ..Default::default()
   }).await?;

   let s3 = vil_storage_s3::process::create_client(S3Config {
       endpoint: Some(std::env::var("S3_ENDPOINT").unwrap()),
       bucket: "uploads".into(),
       ..Default::default()
   }).await?;
   ```
3. **`crates/vil_cli/src/templates.rs`** — update 8 templates to optionally include connector boilerplate

**SDK transpile support:**
Each SDK language (Python, Go, Java, TS, C#, Kotlin, Swift, Zig) gets connector builder methods:

```python
# Python SDK
pipeline = VilPipeline("my-service", port=3080)
pipeline.mongo("primary", uri=env("MONGO_URI"), database="myapp")
pipeline.s3("uploads", endpoint=env("S3_ENDPOINT"), bucket="uploads")
pipeline.rabbitmq("events", uri=env("RABBITMQ_URI"), exchange="events")
pipeline.cron("daily-report", schedule="0 30 6 * * *")
```

This transpiles to the same `app.vil.yaml` → Rust codegen path.

**Estimated effort:** 7-10 days

---

### Stream 4: SDK Templates with Connectors

**Problem:** `vil init --template` only generates HTTP pipeline templates. No template shows how to use databases, storage, MQ, or triggers.

**Solution:** Add 4 new templates to `vil init`:

```
TEMPLATES  (existing 8 + new 4 = 12 total)

Existing:
1. AI Gateway          — SSE streaming pipeline
2. REST CRUD API       — GET/POST/PUT/DELETE endpoints
3. Multi-Model Router  — Route to different LLM providers
4. RAG Pipeline        — Embed → search → generate
5. WebSocket Chat      — WebSocket broadcast
6. WASM FaaS          — WebAssembly functions
7. AI Agent           — ReAct agent with tools
8. Blank Project      — Empty skeleton

New:
9.  Data Pipeline      — S3 ingest → transform → MongoDB store → ClickHouse analytics
10. Event-Driven       — RabbitMQ consume → process → publish result
11. IoT Gateway        — MQTT trigger → validate → Timesceries store → alert
12. Scheduled ETL      — Cron trigger → S3 fetch → transform → Elasticsearch index
```

**Template 9: Data Pipeline**
```yaml
name: data-pipeline
port: 3080
connectors:
  storage:
    - name: ingest
      type: s3
      bucket: raw-data
  databases:
    - name: store
      type: mongo
      database: processed
    - name: analytics
      type: clickhouse
      database: analytics
pipeline:
  source:
    type: http
    path: /trigger
  stages:
    - name: fetch
      connector: ingest
      action: get_object
    - name: transform
      handler: transform_record
    - name: store
      connector: store
      action: insert_one
    - name: analytics
      connector: analytics
      action: insert
```

**Template 10: Event-Driven**
```yaml
name: event-processor
connectors:
  queues:
    - name: input
      type: rabbitmq
      uri: ${RABBITMQ_URI}
      queue: tasks
    - name: output
      type: rabbitmq
      uri: ${RABBITMQ_URI}
      exchange: results
```

**Template 11: IoT Gateway**
```yaml
name: iot-gateway
port: 3080
connectors:
  triggers:
    - name: devices
      type: iot
      mqtt_host: ${MQTT_HOST}
      topic: "sensors/#"
  databases:
    - name: timeseries
      type: timeseries
      influxdb_url: ${INFLUXDB_URL}
      bucket: sensors
```

**Template 12: Scheduled ETL**
```yaml
name: etl-scheduler
connectors:
  triggers:
    - name: schedule
      type: cron
      schedule: "0 0 * * * *"  # hourly
  storage:
    - name: source
      type: s3
      bucket: raw-logs
  databases:
    - name: search
      type: elastic
      url: ${ELASTIC_URL}
```

Each template generates:
- `app.vil.yaml` with connector config
- `src/main.rs` (Rust) or equivalent in other languages
- `handlers/` with stub handler functions
- `README.md` with setup instructions (Docker Compose for required services)

**Estimated effort:** 5-7 days

---

## Implementation Order

```
Week 1:   Stream 1 — vil_connector_macros (proc-macro crate)
          └── #[connector_fault], #[connector_event], #[connector_state]

Week 2:   Stream 2a — Apply macros to Phase 1 crates (storage + DB)
          └── Add events.rs + state.rs to each crate

Week 3:   Stream 2b — Apply macros to Phase 2-3 crates (MQ + triggers)
          └── Add events.rs + state.rs to each crate

Week 4:   Stream 3a — YAML manifest extension
          └── Parse connectors/triggers/logging in manifest.rs

Week 5:   Stream 3b — Rust codegen for connectors
          └── Generate init code from YAML in codegen.rs

Week 6:   Stream 3c — SDK transpile for connectors
          └── Python/Go/Java/TS/C#/Kotlin/Swift/Zig connector methods

Week 7:   Stream 4 — 4 new templates
          └── Data Pipeline, Event-Driven, IoT Gateway, Scheduled ETL

Week 8:   Examples — 1 example per new template per language
          └── 601-604 (DB/storage), 701-704 (MQ/protocol), 801-804 (triggers)
```

---

## Milestone Checklist

### Stream 1: vil_connector_macros
- [ ] `#[connector_fault]` — Display, error_code(), kind(), is_retryable()
- [ ] `#[connector_event]` — repr(C), Debug, Clone, Copy, Default, size guard ≤192B
- [ ] `#[connector_state]` — repr(C), Debug, Clone, Default
- [ ] Zero VIL runtime dependencies (syn + quote only)
- [ ] `cargo check -p vil_connector_macros` passes

### Stream 2: Events & State
- [ ] All 28 connector crates have events.rs + state.rs
- [ ] All fault enums annotated with `#[connector_fault]`
- [ ] All events are `#[repr(C)]`, ≤192 bytes
- [ ] All state structs use atomic counters where appropriate
- [ ] No regressions — all existing tests pass

### Stream 3: YAML Codegen
- [ ] `connectors:` section parsed in manifest.rs
- [ ] `triggers:` section parsed in manifest.rs
- [ ] `logging:` section parsed in manifest.rs
- [ ] Rust codegen generates connector init from YAML
- [ ] `vil compile` works with connector YAML
- [ ] All 9 SDK languages support connector builder methods

### Stream 4: Templates
- [ ] Template 9: Data Pipeline (S3 → Mongo → ClickHouse)
- [ ] Template 10: Event-Driven (RabbitMQ consume → process → publish)
- [ ] Template 11: IoT Gateway (MQTT → TimeSeries → alert)
- [ ] Template 12: Scheduled ETL (Cron → S3 → Elasticsearch)
- [ ] Each template works with `vil init` for all 9 languages
- [ ] Each template includes Docker Compose for dependencies
- [ ] README per template with setup instructions

### Examples
- [ ] 601-604: Storage/DB examples with full semantic types
- [ ] 701-704: MQ/Protocol examples with full semantic types
- [ ] 801-804: Trigger examples with full semantic types
- [ ] All examples use `#[connector_fault]`, events, state
- [ ] All examples compile and demonstrate end-to-end flow
