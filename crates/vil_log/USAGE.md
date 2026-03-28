# vil_log Usage Guide

## When to Use What

### Development Mode (recommended during development)
```rust
// Don't call init_logging() — use standard tracing
tracing_subscriber::fmt().pretty().init();

// All vil_log macros (app_log!, db_log!, etc.) automatically fall back
// to tracing::event!() — familiar output, no setup needed.
app_log!(Info, "order.created", { order_id: 123u64 });
// Output: standard tracing format
```

### Production Mode (recommended for deployment)
```rust
// Call init_logging() — activates high-performance SPSC ring
let _task = init_logging(LogConfig {
    level: LogLevel::Info,
    threads: Some(4),
    ..Default::default()
}, StdoutDrain::resolved());

// Same macro calls — now 4-6x faster
app_log!(Info, "order.created", { order_id: 123u64 });
// Output: 2026-03-28T01:00:00.123Z INFO [App] svc=my-service | order.created {"order_id":123}
```

### Environment-Driven Switch
```rust
if std::env::var("VIL_LOG").unwrap_or("true".into()) == "true" {
    init_logging(LogConfig::default(), StdoutDrain::resolved());
} else {
    tracing_subscriber::fmt().init();
}
// Code below works identically in both modes
```

## Important: Log Schema Backup

**vil_log stores log payloads as raw binary structs, not human-readable text.**

To read old logs in the future, you MUST preserve:
1. **Dictionary file** (`.vil_log_dict.json`) — maps hash→string
2. **Payload struct definitions** — the Rust struct layout at the time logs were written
3. **Schema version** — each LogSlot has a `version` field (currently v1)

### Why This Matters

```
Log written today with DbPayload v1:
  [bytes: 0x1234 0x5678 0x9ABC 0x01C2 ...]

Without the struct definition → meaningless bytes.
With the struct definition → "SELECT mongodb.users dur=450us rows=42"
```

### Recommended Backup Strategy

```bash
# After each release, archive the payload definitions:
cp crates/vil_log/src/types/*.rs log-schema-backup/v0.2.0/

# Dictionary is auto-saved on shutdown, but also backup periodically:
cp .vil_log_dict.json log-schema-backup/dict-$(date +%Y%m%d).json
```

### Schema Evolution

When you change a payload struct in a new version:
1. Bump the version constant in the emitting macro
2. Add a new `resolve_*_detail_v2()` function in resolve.rs
3. Old v1 logs continue to resolve correctly via `resolve_*_detail_v1()`
4. New v2 logs resolve via the new function

## Performance Comparison

| Mode | Latency | Throughput | Use When |
|------|---------|-----------|----------|
| tracing (no init_logging) | ~800ns | ~1.3M/s | Development, debugging |
| vil_log flat (init_logging) | ~130ns | ~7.5M/s | Production, high-throughput |
| vil_log dynamic (app_log!) | ~440ns | ~2.3M/s | Production, business events |
| Filtered out (either mode) | ~0.2ns | ~5000M/s | Below level threshold |
