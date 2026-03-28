# vil_log Drains

Drains are the output backends for the vil_log SPSC ring. Each drain implements the `LogDrain` trait.

## Quick Reference

| Drain | Use Case | Format Options |
|-------|----------|----------------|
| StdoutDrain | Dev / local | pretty, compact, json, resolved |
| FileDrain | Local persistence, rolling | json |
| ClickHouseDrain | Analytics / query | ClickHouse table insert |
| NatsDrain | Distributed log shipping | json over NATS subject |
| MultiDrain | Fan-out to N drains | delegates to children |
| FallbackDrain | Resilience wrapper | primary + fallback |
| NullDrain | Testing / silence | discard all |

## StdoutDrain

```rust
use vil_log::StdoutDrain;

StdoutDrain::pretty()     // human-readable, colorized
StdoutDrain::compact()    // single-line, no color
StdoutDrain::json()       // structured JSON
StdoutDrain::resolved()   // JSON with dict resolution (recommended for prod stdout)
```

## FileDrain

Rolling file drain — rotates by size or time.

```rust
use vil_log::FileDrain;

let drain = FileDrain::new("/var/log/vil/app.jsonl")
    .max_size_mb(256)          // rotate after 256MB
    .max_files(7)              // keep 7 rotated files
    .compress(true)            // gzip rotated files
    .build()?;
```

## ClickHouseDrain

Ships logs directly to ClickHouse for analytics queries.

```rust
use vil_log::ClickHouseDrain;
use vil_db_clickhouse::ClickHousePool;

let ch = ClickHousePool::new()
    .url("http://localhost:8123")
    .database("vil_logs")
    .build()
    .await?;

let drain = ClickHouseDrain::new(ch)
    .table("app_logs")
    .batch_size(1000)
    .flush_interval_ms(500)
    .build();
```

## NatsDrain

Publishes log entries to a NATS subject for distributed log aggregation.

```rust
use vil_log::NatsDrain;
use vil_mq_nats::NatsClient;

let nats = NatsClient::new().url("nats://localhost:4222").build().await?;

let drain = NatsDrain::new(nats)
    .subject("logs.vil.{service}")   // {service} replaced at runtime
    .format(NatsDrainFormat::Json)
    .build();
```

## MultiDrain

Fan-out to multiple drains simultaneously.

```rust
use vil_log::MultiDrain;

let drain = MultiDrain::new(vec![
    Box::new(StdoutDrain::compact()),
    Box::new(FileDrain::new("/var/log/vil/app.jsonl").build()?),
    Box::new(ClickHouseDrain::new(ch_pool).build()),
]);
```

## FallbackDrain

Switches to fallback drain if primary drain fails N consecutive times.

```rust
use vil_log::FallbackDrain;

let drain = FallbackDrain::new(
    Box::new(ClickHouseDrain::new(ch_pool).build()),  // primary
    Box::new(FileDrain::new("/tmp/vil_fallback.jsonl").build()?), // fallback
)
.failure_threshold(3);  // switch after 3 consecutive errors
```

This is also configurable via `LogConfig::drain_failure_threshold` when using `init_logging`.

## NullDrain

Silently discards all log entries. Useful in tests.

```rust
use vil_log::NullDrain;

init_logging(LogConfig::default(), NullDrain);
```

## Common Patterns

### Production: ClickHouse + Fallback to File
```rust
init_logging(
    LogConfig { ring_slots: 1 << 20, level: LogLevel::Info, ..Default::default() },
    FallbackDrain::new(
        Box::new(ClickHouseDrain::new(ch).build()),
        Box::new(FileDrain::new("/var/log/vil/fallback.jsonl").build()?),
    ).failure_threshold(3)
);
```

### Dev: Stdout pretty
```rust
// Don't call init_logging → auto tracing fallback
// OR explicitly:
init_logging(LogConfig::default(), StdoutDrain::pretty());
```

### Ship to NATS + archive to file
```rust
init_logging(
    LogConfig { ring_slots: 1 << 20, ..Default::default() },
    MultiDrain::new(vec![
        Box::new(NatsDrain::new(nats).subject("logs.app").build()),
        Box::new(FileDrain::new("/archive/app.jsonl").build()?),
    ])
);
```
