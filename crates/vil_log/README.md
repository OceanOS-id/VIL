# vil_log — VIL Semantic Log System

Zero-copy, non-blocking, structured logging for high-throughput VIL pipelines.

## Why not tracing?

| | tracing (fmt + NonBlocking) | VIL access_log! (flat) | VIL app_log! (dynamic) |
|---|---|---|---|
| **ns/event** | ~600 | ~130 | ~350 |
| **M events/s** | 1.7 | 7.5 | 2.9 |
| **Speedup** | baseline | **4.5x faster** | **1.7x faster** |

VIL Log achieves this by:
- **No string formatting** on hot path — format later, on drain thread
- **No heap allocation** — fixed 256-byte slots in pre-allocated SPSC ring
- **No lock contention** — single atomic CAS per push (SPSC, not MPMC)
- **Ring full?** Drop + count. Never block the caller.

## Architecture

```
HOT PATH (~35-140ns)                 COLD PATH (async)
────────────────────                 ─────────────────────────────
app_log!() ──┐                       ┌─► StdoutDrain (pretty/json)
access_log!()┤                       ├─► FileDrain (rolling)
ai_log!()    ├─► SpscRing ───────────┼─► ClickHouseDrain (batch)
db_log!()    │   (lock-free,         ├─► NatsDrain (fan-out)
mq_log!()    │    256B slots)        └─► MultiDrain (N drains)
tracing ─────┘
  (bridge)
```

## Quick Start

```rust
use vil_log::prelude::*;
use vil_log::drain::StdoutDrain;
use vil_log::runtime::init_logging;

#[tokio::main]
async fn main() {
    // Initialize with stdout drain
    let config = LogConfig {
        ring_slots: 16384,          // 16K slots = 4MB ring
        level: LogLevel::Info,
        batch_size: 1024,
        flush_interval_ms: 100,
    };
    init_logging(config, StdoutDrain::pretty());

    // Structured business event
    app_log!(Info, "order.created", {
        order_id: 12345u64,
        amount: 50000u64,
    });

    // Flat struct — zero serialization, pure memcpy
    access_log!(Info, AccessPayload {
        method: 1,  // POST
        status_code: 201,
        duration_us: 2300,
        path_hash: vil_log::dict::register_str("/api/orders"),
        ..Default::default()
    });
}
```

## Development vs Production

**During development**, don't call `init_logging()`. All macros fall back to standard `tracing` — familiar output, zero setup:

```rust
tracing_subscriber::fmt().pretty().init();
app_log!(Info, "order.created", { order_id: 123u64 });
// → standard tracing pretty output
```

**In production**, call `init_logging()` for 4-6x faster logging:

```rust
init_logging(LogConfig::default(), StdoutDrain::resolved());
app_log!(Info, "order.created", { order_id: 123u64 });
// → 2026-03-28 INFO [App] svc=my-service | order.created {"order_id":123}
```

**Important:** vil_log stores binary payloads. Back up `.vil_log_dict.json` and `crates/vil_log/src/types/*.rs` with each release to ensure old logs remain readable. See [USAGE.md](USAGE.md) for details.

## 7 Semantic Log Types

| Macro | Category | Layout | Use Case |
|-------|----------|--------|----------|
| `app_log!` | App | MsgPack KV | Business logic events |
| `access_log!` | Access | Flat struct | HTTP request/response |
| `ai_log!` | AI | Flat struct | LLM/RAG/Agent operations |
| `db_log!` | DB | Flat struct | Database queries |
| `mq_log!` | MQ | Flat struct | Message queue pub/sub |
| `system_log!` | System | Flat struct | Runtime internals (SHM, queue, process) |
| `security_log!` | Security | Flat struct | Auth, rate limit, injection |

**Flat struct** macros are fastest (~130ns) — pure memcpy, no serialization.
**app_log!** is slower (~350ns) due to MsgPack encoding of dynamic KV pairs.

## Drain Backends

### Built-in (always available)

| Drain | Use Case |
|-------|----------|
| `StdoutDrain` | Dev mode — pretty/compact/json format |
| `FileDrain` | Rolling file — daily/hourly/size rotation, retention |
| `NullDrain` | Benchmarks — discard everything |
| `MultiDrain` | Fan-out to N drains simultaneously |

### Feature-gated

| Drain | Feature Flag | Use Case |
|-------|-------------|----------|
| `ClickHouseDrain` | `clickhouse-drain` | Batch INSERT for analytics |
| `NatsDrain` | `nats-drain` | Cross-host log aggregation |

```toml
# Enable ClickHouse drain
vil_log = { workspace = true, features = ["clickhouse-drain"] }

# Enable NATS drain
vil_log = { workspace = true, features = ["nats-drain"] }

# Enable all
vil_log = { workspace = true, features = ["all-drains"] }
```

## Multi-Drain Example

```rust
use vil_log::drain::*;

let drain = MultiDrain::new()
    .add(StdoutDrain::compact())
    .add(FileDrain::new(FileConfig {
        dir: "/var/log/vil".into(),
        rotation: RotationStrategy::Daily,
        max_files: 30,
        ..Default::default()
    }));

init_logging(config, drain);
```

## Level Filtering

Events below the configured level are filtered **before** touching the ring — cost is a single atomic load (~1ns).

```rust
let config = LogConfig {
    level: LogLevel::Info,  // Debug and Trace are discarded
    ..Default::default()
};
```

## tracing Bridge

VIL Log can capture events from the Rust `tracing` ecosystem:

```rust
use vil_log::runtime::init_logging_with_tracing;

// This installs VilTracingLayer as the global tracing subscriber.
// All tracing::info!(), tracing::warn!(), etc. flow into the VIL ring.
init_logging_with_tracing(config, StdoutDrain::pretty());
```

## Dictionary System

Hot-path logs use `u32` hashes instead of strings. Register strings once:

```rust
use vil_log::dict::register_str;

let hash = register_str("/api/orders");  // returns u32
// Use hash in AccessPayload.path_hash, etc.

// Reverse lookup (cold path, for drain formatting)
let original = vil_log::dict::lookup(hash);
```

## Examples

```bash
cargo run -p example-501-villog-stdout-dev         # Stdout pretty output
cargo run -p example-502-villog-file-rolling        # Rolling file drain
cargo run -p example-503-villog-multi-drain         # Multi-drain fan-out
cargo run -p example-504-villog-benchmark-comparison --release  # Benchmark
cargo run -p example-505-villog-tracing-bridge      # tracing ecosystem bridge
cargo run -p example-506-villog-structured-events   # All 7 log types
```

## License

Apache-2.0
