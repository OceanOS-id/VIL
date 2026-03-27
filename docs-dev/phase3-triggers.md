# Phase 3 — Q1 2027: Trigger & Event Source

> **⚠ MANDATORY: Read [COMPLIANCE.md](./COMPLIANCE.md) before implementing any crate in this phase.**
> Every crate must pass the full compliance checklist (P1–P10, testing, docs, pre-merge review).
> Non-compliant crates will be rejected regardless of functionality.

## Objective

Introduce a unified trigger system that turns external events (time, file changes, database mutations, emails, IoT signals, blockchain logs) into Tri-Lane pipeline activations. Each trigger is a `ServiceProcess` emitting on Trigger Lane.

---

## Shared Trait: `TriggerSource`

Defined in a new `vil_trigger_core` crate (or in `vil_types`):

```rust
#[async_trait]
pub trait TriggerSource: Send + Sync {
    /// Unique trigger type identifier
    fn kind(&self) -> &'static str;

    /// Start watching for events, emitting on Trigger Lane
    async fn start(&self, emitter: TriLaneEmitter) -> Result<()>;

    /// Pause without destroying state
    async fn pause(&self) -> Result<()>;

    /// Resume after pause
    async fn resume(&self) -> Result<()>;

    /// Graceful shutdown, cleanup resources
    async fn stop(&self) -> Result<()>;
}
```

### Tri-Lane mapping (all triggers):

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Outbound → Pipeline | Event descriptor (lightweight, zero-copy) |
| Data | Outbound → Pipeline | Event payload (if any, via ExchangeHeap) |
| Control | Inbound ← Pipeline | Pause / Resume / Stop / Reconfigure |

---

## 1. `vil_trigger_core` — Shared Trigger Infrastructure

**Priority**: Prerequisite (build first)

**Scope**:
- `TriggerSource` trait (above)
- `TriLaneEmitter` — writes event descriptors to Trigger Lane
- `TriggerRegistry` — register/deregister triggers at runtime
- `TriggerConfig` — YAML-based trigger configuration
- `TriggerProcess` — `ServiceProcess` wrapper for any `TriggerSource`

**Implementation Plan**:
```
crates/vil_trigger_core/
├── src/
│   ├── lib.rs
│   ├── traits.rs       — TriggerSource trait
│   ├── emitter.rs      — TriLaneEmitter (Trigger Lane writer)
│   ├── registry.rs     — TriggerRegistry (dynamic add/remove)
│   ├── config.rs       — YAML trigger configuration parser
│   ├── process.rs      — TriggerProcess (ServiceProcess wrapper)
│   └── types.rs        — #[vil_event] TriggerFired, #[vil_fault] TriggerFault
├── tests/
└── examples/
```

**Semantic types**:
```rust
#[vil_event(layout = "flat")]
pub struct TriggerFired {
    pub trigger_id: u64,
    pub kind_hash: u64,       // hash of trigger kind string
    pub timestamp_ns: u64,
    pub sequence: u64,
}

#[vil_fault]
pub enum TriggerFault {
    SourceUnavailable { trigger_id: u64, reason_code: u32 },
    ConfigInvalid { trigger_id: u64, field_hash: u64 },
    RateLimited { trigger_id: u64, events_per_sec: u64 },
}
```

**Estimated effort**: 3 days

---

## 2. `vil_trigger_cron` — Cron / Schedule Trigger

**Priority**: High (most universally needed)

**Scope**:
- Cron expression parser (standard 5-field + extended 6-field with seconds)
- Fixed interval scheduler (every N seconds/minutes/hours)
- Timezone-aware scheduling
- Missed-fire policy: skip / fire-immediately / queue
- One-shot timer support

**Dependencies**:
- `cron` crate (cron expression parsing)
- `chrono` or `jiff` (timezone)
- `vil_trigger_core`

**Implementation Plan**:
```
crates/vil_trigger_cron/
├── src/
│   ├── lib.rs
│   ├── source.rs       — CronTriggerSource implements TriggerSource
│   ├── parser.rs       — cron expression + interval parsing
│   ├── scheduler.rs    — next-fire calculation, timezone handling
│   ├── policy.rs       — missed-fire policies
│   └── error.rs        — #[vil_fault] CronFault
├── tests/
│   ├── unit.rs         — parser + scheduler tests
│   └── integration.rs  — real timer tests
└── examples/
    └── cron_scheduled_pipeline.rs
```

**YAML config example**:
```yaml
triggers:
  - kind: cron
    id: daily-report
    schedule: "0 30 6 * * *"    # 06:30 daily
    timezone: Asia/Jakarta
    missed_fire: fire_immediately
```

**Estimated effort**: 2-3 days

---

## 3. `vil_trigger_fs` — File / S3 Watcher Trigger

**Priority**: High (data pipeline use case)

**Scope**:
- Local filesystem watcher (inotify on Linux, FSEvents on macOS)
- S3-compatible watcher (polling ListObjectsV2 with ETag tracking)
- Event types: Created, Modified, Deleted, Renamed
- Glob pattern filter (e.g., `*.csv`, `data/**/*.parquet`)
- Debounce (configurable, default 500ms)
- Initial scan option (fire for existing files on start)

**Dependencies**:
- `notify` crate (cross-platform fs watcher)
- `vil_storage_s3` (for S3 watching — Phase 1 dependency)
- `vil_trigger_core`

**Implementation Plan**:
```
crates/vil_trigger_fs/
├── src/
│   ├── lib.rs
│   ├── local.rs        — LocalFsWatcher implements TriggerSource
│   ├── s3.rs           — S3Watcher implements TriggerSource (polling)
│   ├── filter.rs       — glob pattern matching
│   ├── debounce.rs     — event debouncing
│   ├── types.rs        — #[vil_event] FsEvent { path_hash, event_kind, size, ... }
│   └── error.rs
├── tests/
└── examples/
    ├── watch_local_dir.rs
    └── watch_s3_bucket.rs
```

**Semantic types**:
```rust
#[vil_event(layout = "flat")]
pub struct FsEvent {
    pub trigger_id: u64,
    pub path_hash: u64,         // hash of file path
    pub event_kind: u8,         // 0=Created, 1=Modified, 2=Deleted, 3=Renamed
    pub size_bytes: u64,
    pub timestamp_ns: u64,
}
```

**Estimated effort**: 3-4 days

---

## 4. `vil_trigger_cdc` — Database Change Data Capture

**Priority**: High (event-driven architecture)

**Scope**:
- PostgreSQL logical replication (pgoutput plugin)
- MySQL binlog consumer
- Debezium-compatible JSON format (for interop with existing CDC pipelines)
- Table/column filtering
- Snapshot mode (initial full load) + streaming mode
- LSN/GTID tracking for resume after restart

**Dependencies**:
- `tokio-postgres` (logical replication protocol)
- `mysql_async` (binlog API)
- `vil_trigger_core`, `vil_json`

**Implementation Plan**:
```
crates/vil_trigger_cdc/
├── src/
│   ├── lib.rs
│   ├── postgres.rs     — PostgresCdcSource (logical replication)
│   ├── mysql.rs        — MysqlCdcSource (binlog)
│   ├── debezium.rs     — Debezium JSON format adapter
│   ├── filter.rs       — table/column inclusion/exclusion
│   ├── checkpoint.rs   — LSN/GTID tracking + persistence
│   ├── types.rs        — #[vil_event] CdcEvent { op, table_hash, before, after }
│   └── error.rs
├── tests/
│   └── integration.rs  — Docker PostgreSQL + MySQL with CDC enabled
└── examples/
    ├── postgres_cdc_pipeline.rs
    └── mysql_cdc_pipeline.rs
```

**Semantic types**:
```rust
#[vil_event(layout = "relative")]
pub struct CdcEvent {
    pub source_hash: u64,       // hash of source identifier
    pub table_hash: u64,        // hash of table name
    pub operation: u8,          // 0=Insert, 1=Update, 2=Delete
    pub lsn: u64,               // log sequence number
    pub timestamp_ns: u64,
    pub payload: VSlice<u8>,    // before/after row data in ExchangeHeap
}
```

**Estimated effort**: 7-10 days (complex — two DB engines + checkpoint)

---

## 5. `vil_trigger_email` — Email Trigger (IMAP/SMTP)

**Priority**: Medium

**Scope**:
- IMAP IDLE listener (push-based, no polling)
- Folder/label filter
- Subject/sender regex filter
- Attachment extraction (stream to ExchangeHeap)
- SMTP send as response action (optional, separate from trigger)

**Dependencies**:
- `async-imap` + `async-native-tls`
- `mail-parser` (MIME parsing)
- `vil_trigger_core`

**Implementation Plan**:
```
crates/vil_trigger_email/
├── src/
│   ├── lib.rs
│   ├── imap.rs         — ImapTriggerSource (IDLE-based)
│   ├── filter.rs       — subject/sender/folder filters
│   ├── parser.rs       — MIME parsing, attachment extraction
│   ├── smtp.rs         — optional SMTP sender (response action)
│   ├── types.rs        — #[vil_event] EmailEvent
│   └── error.rs
├── tests/
│   └── integration.rs  — Docker GreenMail IMAP server
└── examples/
    └── email_triggered_pipeline.rs
```

**Estimated effort**: 4-5 days

---

## 6. `vil_trigger_iot` — IoT Device Event Trigger

**Priority**: Medium (smart city, industrial)

**Scope**:
- MQTT subscription-based trigger (reuses `vil_mq_mqtt` internally)
- CoAP observe support
- Device registry (track known devices, detect new/offline)
- Payload normalization (heterogeneous devices → uniform VIL event)
- Heartbeat / offline detection

**Dependencies**:
- `vil_mq_mqtt` (MQTT subscription)
- `coap-lite` (CoAP protocol)
- `vil_trigger_core`

**Implementation Plan**:
```
crates/vil_trigger_iot/
├── src/
│   ├── lib.rs
│   ├── mqtt.rs         — MqttIotTrigger (wraps vil_mq_mqtt consumer)
│   ├── coap.rs         — CoapObserveTrigger
│   ├── registry.rs     — device registry + offline detection
│   ├── normalize.rs    — heterogeneous payload → uniform event
│   ├── types.rs        — #[vil_event] IotDeviceEvent
│   └── error.rs
├── tests/
└── examples/
    └── iot_sensor_pipeline.rs
```

**Estimated effort**: 4-5 days

---

## 7. `vil_trigger_evm` — Blockchain Event Trigger (EVM)

**Priority**: Low-Medium (niche but growing)

**Scope**:
- Ethereum JSON-RPC `eth_subscribe` (logs)
- Event ABI decoding (topic + data → structured event)
- Block confirmation threshold (configurable finality)
- Multi-chain support (Ethereum, Polygon, BSC, Arbitrum — same interface)
- Reconnect with block catchup (no missed events)

**Dependencies**:
- `alloy` (Ethereum library — successor to ethers-rs)
- `vil_trigger_core`

**Implementation Plan**:
```
crates/vil_trigger_evm/
├── src/
│   ├── lib.rs
│   ├── subscriber.rs   — EvmLogSubscriber implements TriggerSource
│   ├── abi.rs          — ABI event decoding
│   ├── filter.rs       — contract address + event signature filter
│   ├── finality.rs     — block confirmation tracking
│   ├── catchup.rs      — missed-block recovery on reconnect
│   ├── types.rs        — #[vil_event] EvmLogEvent
│   └── error.rs
├── tests/
│   └── integration.rs  — Hardhat/Anvil local chain
└── examples/
    └── evm_event_pipeline.rs
```

**Semantic types**:
```rust
#[vil_event(layout = "relative")]
pub struct EvmLogEvent {
    pub chain_id: u64,
    pub block_number: u64,
    pub tx_hash: [u8; 32],
    pub log_index: u32,
    pub contract_hash: u64,     // hash of contract address
    pub topic0: [u8; 32],       // event signature
    pub data: VSlice<u8>,       // decoded event data in ExchangeHeap
    pub timestamp_ns: u64,
}
```

**Estimated effort**: 5-6 days

---

## 8. `vil_trigger_webhook` — Webhook Enrichment

**Priority**: Medium (already exists partially in vil_server, this formalizes it)

**Scope**:
- Webhook receiver as `TriggerSource`
- HMAC signature verification (GitHub, Stripe, Slack, custom)
- Request transformation (headers → event metadata)
- Retry tracking (idempotency key dedup)
- Rate limiting per source

**Dependencies**:
- `vil_server_core` (HTTP endpoint)
- `vil_server_auth` (HMAC, rate limiting)
- `vil_trigger_core`

**Implementation Plan**:
```
crates/vil_trigger_webhook/
├── src/
│   ├── lib.rs
│   ├── receiver.rs     — WebhookTriggerSource (HTTP endpoint → TriggerSource)
│   ├── verify.rs       — HMAC verification (multi-provider)
│   ├── transform.rs    — request → event transformation
│   ├── dedup.rs        — idempotency key tracking
│   ├── types.rs        — #[vil_event] WebhookEvent
│   └── error.rs
├── tests/
└── examples/
    └── github_webhook_pipeline.rs
```

**Estimated effort**: 3 days

---

## Development Order

Build sequentially due to dependencies:

1. **`vil_trigger_core`** — shared infrastructure (week 1)
2. **`vil_trigger_cron`** — simplest, validates core design (week 1)
3. **`vil_trigger_webhook`** — builds on existing server infra (week 2)
4. **`vil_trigger_fs`** — depends on Phase 1 `vil_storage_s3` (week 2)
5. **`vil_trigger_cdc`** — most complex, needs dedicated focus (week 3-4)
6. **`vil_trigger_email`** — independent (week 4)
7. **`vil_trigger_iot`** — depends on `vil_mq_mqtt` (week 5)
8. **`vil_trigger_evm`** — niche, can be last (week 5)

---

## Milestone Checklist

- [ ] `vil_trigger_core` — trait + emitter + registry + process wrapper
- [ ] `vil_trigger_cron` — cron + interval + timezone + missed-fire
- [ ] `vil_trigger_fs` — local fs + S3 watcher + glob filter
- [ ] `vil_trigger_cdc` — PostgreSQL + MySQL + Debezium format
- [ ] `vil_trigger_email` — IMAP IDLE + MIME + attachment extraction
- [ ] `vil_trigger_iot` — MQTT + CoAP + device registry
- [ ] `vil_trigger_evm` — EVM log subscription + ABI decode + finality
- [ ] `vil_trigger_webhook` — HMAC verify + transform + dedup
- [ ] All triggers implement `TriggerSource` trait
- [ ] All triggers expose Tri-Lane (Trigger/Data/Control)
- [ ] All `#[vil_event]` types use Flat or Relative layout (no heap)
- [ ] All `#[vil_fault]` types for errors (no `thiserror`)
- [ ] COMPLIANCE.md checklist passed for every crate
- [ ] `vil init` updated with trigger-enabled templates
