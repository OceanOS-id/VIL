# vil_db_clickhouse

VIL Database Plugin — ClickHouse OLAP with batch INSERT and semantic logging.

## Overview

`vil_db_clickhouse` provides a thin, allocation-minimal async client for
[ClickHouse](https://clickhouse.com/) built on top of the official
[`clickhouse`](https://crates.io/crates/clickhouse) Rust crate.

Every operation automatically emits a `db_log!` event via `vil_log`, capturing
timing, row counts, and hashed identifiers — no `println!`, `tracing::info!`,
or manual metrics required.

## Features

- **`ChClient`** — single-connection async client with `query`, `execute`, and
  `insert` methods. All string fields are stored as `u32` FxHashes via
  `register_str()`.
- **`BatchInserter<T>`** — buffered inserter with configurable flush-by-row-count
  and flush-by-age policies.
- **`ChFault`** — allocation-free error type; all string fields are `u32` hashes.
- **`db_log!` auto-emit** — every query/insert wraps `Instant::now()` and emits
  a `DbPayload` with `duration_us` and `rows_affected`.

## Quick Start

```rust,no_run
use vil_db_clickhouse::{ChClient, ClickHouseConfig};

#[tokio::main]
async fn main() {
    let client = ChClient::new(ClickHouseConfig {
        url: "http://localhost:8123".to_string(),
        database: "analytics".to_string(),
        username: Some("default".to_string()),
        password: None,
    });

    // DDL
    client
        .execute("CREATE TABLE IF NOT EXISTS events (ts UInt64, msg String) ENGINE = MergeTree() ORDER BY ts")
        .await
        .expect("DDL failed");
}
```

## Batch Insert

```rust,no_run
use std::time::Duration;
use vil_db_clickhouse::{BatchInserter, ChClient, ClickHouseConfig};
use clickhouse::Row;
use serde::Serialize;

#[derive(Row, Serialize)]
struct Event {
    ts: u64,
    msg: String,
}

#[tokio::main]
async fn main() {
    let client = ChClient::new(ClickHouseConfig::default());
    let mut batch: BatchInserter<Event> =
        BatchInserter::new(client, "events", 1000, Duration::from_secs(5));

    for i in 0u64..2000 {
        batch.push(Event { ts: i, msg: format!("row {i}") }).await.unwrap();
    }

    // Drain remaining rows at shutdown
    batch.flush().await.unwrap();
}
```

## Compliance

| Requirement | Status |
|---|---|
| `vil_log = { workspace = true }` in `Cargo.toml` | ✓ |
| `db_log!` auto-emit on every operation | ✓ |
| `register_str()` for all string fields | ✓ |
| No `println!` / `eprintln!` / `tracing::info!` | ✓ |
| `ChFault` is allocation-free (no `String` fields) | ✓ |
| Boundary classification: Copy at network, zero-copy internal | ✓ |

## Boundary Classification

| Path | Mode | Notes |
|---|---|---|
| ClickHouse wire protocol | Copy | HTTP/RowBinary serialization required |
| Internal `Vec<T>` buffer | Heap (setup/batch) | Acceptable — batch aggregation path |
| `db_log!` emission | Zero-copy | `DbPayload` is `Copy`; pushed into ring without allocation |
| `ChFault` propagation | Copy | Enum with integer fields only |

## Tri-Lane Mapping (Database Connector)

| Lane | Direction | Content |
|---|---|---|
| Trigger | Inbound → VIL | Query request descriptor |
| Data | Outbound ← VIL | Query result set |
| Control | Bidirectional | Transaction commit/rollback, connection error |

## Thread Hint

`ChClient` spawns no threads directly. The underlying `clickhouse::Client`
HTTP pool uses tokio tasks managed by the caller's runtime. No additional
`LogConfig.threads` budget is required beyond the application's existing
tokio worker count.
