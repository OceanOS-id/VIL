# vil_log — Semantic Log System

## Quick Reference
- 7 semantic types: AccessLog, AppLog, AiLog, DbLog, MqLog, SystemLog, SecurityLog
- Hot path: ~130ns (flat types), ~440ns (dynamic app_log)
- 4-6x faster than Rust's tracing
- Auto-emit from #[vil_handler], vil_llm, vil_db_*, vil_mq_*, vil_rt
- Striped SPSC rings, auto-sized to CPU cores

## Usage

```rust
use vil_log::{app_log, LogConfig, LogLevel, StdoutDrain};
use vil_log::runtime::init_logging;

// Production: init_logging → SPSC ring (130ns)
init_logging(LogConfig {
    ring_slots: 1 << 20,
    level: LogLevel::Info,
    threads: Some(4),
    ..Default::default()
}, StdoutDrain::resolved());

// Dev: DON'T call init_logging → auto tracing fallback (800ns)

app_log!(Info, "order.created", { order_id: 123u64 });
db_log!(Info, DbPayload { duration_us: 450, ..Default::default() });
```

## All 7 Macros

| Macro | Auto-emitted by | Category |
|-------|----------------|----------|
| access_log! | #[vil_handler] | HTTP req/res |
| ai_log! | vil_llm providers | LLM calls |
| db_log! | vil_db_* | DB queries |
| mq_log! | vil_mq_* | MQ pub/sub |
| system_log! | vil_rt | Process lifecycle |
| security_log! | manual | Auth events |
| app_log! | manual | Business logic |

## Drains

StdoutDrain (pretty/compact/json/resolved), FileDrain (rolling), ClickHouseDrain, NatsDrain, MultiDrain, FallbackDrain, NullDrain

See [drains.md](drains.md) for full configuration examples.

## Key Config

```rust
LogConfig {
    ring_slots: 1 << 20,          // total capacity
    level: LogLevel::Info,         // filter
    threads: Some(4),              // stripe count (None = auto)
    dict_path: None,               // .vil_log_dict.json
    fallback_path: None,           // .vil_log_fallback.jsonl
    drain_failure_threshold: 3,    // switch to fallback after N failures
    ..Default::default()
}
```

## Common Patterns

### Manual app_log with structured fields
```rust
app_log!(Warn, "payment.retry", {
    order_id: order.id,
    attempt: retry_count,
    reason: "gateway_timeout"
});
```

### Security log for auth events
```rust
security_log!(Warn, SecurityPayload {
    user_id: claims.sub.clone(),
    action: "login.failed",
    ip: remote_addr.to_string(),
    ..Default::default()
});
```

### Combining with MultiDrain
```rust
init_logging(
    LogConfig { ring_slots: 1 << 20, ..Default::default() },
    MultiDrain::new(vec![
        Box::new(StdoutDrain::compact()),
        Box::new(ClickHouseDrain::new(ch_pool)),
    ])
);
```

## Performance Benchmarks

| Operation | Latency | Notes |
|-----------|---------|-------|
| access_log! (flat) | ~130ns | Pre-allocated SHM slot |
| db_log! (flat) | ~130ns | Pre-allocated SHM slot |
| app_log! (dynamic) | ~440ns | Heap alloc for fields |
| tracing::info! | ~600-800ns | Comparison baseline |
| init_logging skip (dev) | ~800ns | tracing fallback |
