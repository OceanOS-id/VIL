# vil_log: Development vs Production

## Quick Reference

| Mode | init_logging | Latency | Output |
|------|-------------|---------|--------|
| Development | NOT called | ~800ns | tracing subscriber (human-readable) |
| Production | Called | ~130ns | SPSC ring → configured drain |

## Development Mode

Do NOT call `init_logging`. vil_log auto-detects the absence of the SPSC ring and falls back to Rust's `tracing` subscriber.

```rust
// Cargo.toml
[dev-dependencies]
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

// main.rs (dev binary / tests)
tracing_subscriber::fmt()
    .with_env_filter("info,my_crate=debug")
    .init();

// No init_logging call → vil_log uses tracing fallback automatically
app_log!(Info, "order.created", { order_id: 42u64 });
// → emits via tracing::info! under the hood
```

### Why tracing fallback in dev?
- Human-readable console output with colors
- Compatible with `RUST_LOG` env var
- Works with standard tracing tooling (tokio-console, jaeger)
- No SPSC ring overhead during local iteration

## Production Mode

Call `init_logging` once at startup, before any async runtime:

```rust
use vil_log::{app_log, LogConfig, LogLevel, StdoutDrain};
use vil_log::runtime::init_logging;

fn main() {
    // Must be called before tokio::main or VilApp::run
    init_logging(
        LogConfig {
            ring_slots: 1 << 20,          // 1M slots
            level: LogLevel::Info,
            threads: Some(4),              // SPSC stripe count
            drain_failure_threshold: 3,
            ..Default::default()
        },
        StdoutDrain::resolved(),           // or ClickHouseDrain, MultiDrain, etc.
    );

    // Now all vil_log macros use the SPSC ring (~130ns)
    VilApp::new("my-service").run().await;
}
```

## Behavior Matrix

| Scenario | init_logging | SPSC Ring | Drain | Fallback File |
|----------|-------------|-----------|-------|---------------|
| Dev (default) | No | No | tracing | No |
| Prod minimal | Yes | Yes | StdoutDrain | No |
| Prod full | Yes | Yes | ClickHouseDrain | .vil_log_fallback.jsonl |
| Test (silence) | Yes (NullDrain) | Yes | NullDrain | No |

## Fallback File

When a drain fails `drain_failure_threshold` times consecutively, vil_log writes to `.vil_log_fallback.jsonl` in the working directory (configurable via `LogConfig::fallback_path`).

```rust
LogConfig {
    fallback_path: Some("/var/log/vil/emergency.jsonl".into()),
    drain_failure_threshold: 3,
    ..Default::default()
}
```

## Dictionary Compression

vil_log supports string dictionary compression for repeated field names/values:

```rust
LogConfig {
    dict_path: Some(".vil_log_dict.json".into()),
    ..Default::default()
}
```

The dictionary is built at startup and resolves field name indices to strings, reducing per-entry allocation. `StdoutDrain::resolved()` decompresses on output.

## Common Patterns

### Feature-flag switching (dev vs prod)
```rust
#[cfg(debug_assertions)]
fn setup_logging() {
    tracing_subscriber::fmt().with_env_filter("debug").init();
}

#[cfg(not(debug_assertions))]
fn setup_logging() {
    init_logging(
        LogConfig { ring_slots: 1 << 20, level: LogLevel::Info, ..Default::default() },
        StdoutDrain::json(),
    );
}
```

### Environment-driven drain selection
```rust
let drain: Box<dyn LogDrain> = match std::env::var("VIL_LOG_DRAIN").as_deref() {
    Ok("clickhouse") => Box::new(ClickHouseDrain::new(ch).build()),
    Ok("file")       => Box::new(FileDrain::new("/var/log/vil/app.jsonl").build()?),
    _                => Box::new(StdoutDrain::json()),
};
init_logging(LogConfig::default(), drain);
```
