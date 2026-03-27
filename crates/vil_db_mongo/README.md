# vil_db_mongo

VIL Database Plugin — MongoDB with semantic logging and Tri-Lane bridge.

## Overview

`vil_db_mongo` wraps the official [mongodb](https://crates.io/crates/mongodb) driver
with full VIL semantic log integration. Every CRUD operation automatically emits a
`db_log!` entry into the VIL log ring — zero `println!`, zero `tracing::info!`.

## Compliance

| Section | Requirement | Status |
|---------|-------------|--------|
| §8 Semantic Log | `vil_log` dependency, `db_log!` on every op | Done |
| §8 Semantic Log | No `println!` / `tracing` / `log` | Done |
| §8 Semantic Log | `register_str()` for all hash fields | Done |
| §8 Semantic Log | `Instant::now()` timing on every op | Done |
| §10 Docs | README + Cargo.toml metadata | Done |
| §11 Crate Structure | config / client / crud / error / types layout | Done |

## Boundary Classification

| Path | Mode | Notes |
|------|------|-------|
| Network I/O (MongoDB wire) | Copy | Required by BSON/TCP protocol |
| Internal pipeline path | Copy | Phase 0 — SHM bridge deferred to future phase |
| Config/metadata | Copy | Setup-time, not hot-path |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Query request descriptor |
| Data | Outbound ← VIL | Query result set |
| Control | Bidirectional | Connection error / transaction commit/rollback |

## Quick Start

```rust
use vil_db_mongo::{MongoClient, MongoConfig};
use bson::doc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    name: String,
    age: u32,
}

#[tokio::main]
async fn main() {
    // Initialize VIL log ring first (in your application entry point)
    // let _task = vil_log::init_logging(vil_log::LogConfig::default(), vil_log::StdoutDrain::pretty());

    let config = MongoConfig::new("mongodb://localhost:27017", "myapp");
    let client = MongoClient::new(config).await.expect("connect to MongoDB");

    // Insert
    let id = client
        .insert_one("users", &User { name: "Alice".into(), age: 30 })
        .await
        .unwrap();

    // Find one
    let user: Option<User> = client
        .find_one("users", doc! { "_id": &id })
        .await
        .unwrap();

    // Find many (limit 10)
    let users: Vec<User> = client
        .find_many("users", doc! {}, Some(10))
        .await
        .unwrap();

    // Update
    let modified = client
        .update_one("users", doc! { "_id": &id }, doc! { "$set": { "age": 31 } })
        .await
        .unwrap();

    // Count
    let total = client.count("users", None).await.unwrap();

    // Delete
    let deleted = client
        .delete_one("users", doc! { "_id": &id })
        .await
        .unwrap();
}
```

## Configuration

```rust
use vil_db_mongo::MongoConfig;

let config = MongoConfig {
    uri: "mongodb://user:pass@host:27017".into(),
    database: "production".into(),
    min_pool: Some(2),
    max_pool: Some(32),
};
```

### YAML equivalent

```yaml
uri: "mongodb://localhost:27017"
database: "myapp"
min_pool: 2
max_pool: 16
```

## Thread Hint for LogConfig

This crate delegates all async I/O to the mongodb driver's internal connection pool.
Add `max_pool` (default: driver-determined) to your `LogConfig.threads` estimate for
optimal log ring sizing.

## Log Fields Emitted

Every operation emits a `DbPayload` with:

| Field | Value |
|-------|-------|
| `db_hash` | FxHash of the database name |
| `table_hash` | FxHash of the collection name |
| `duration_us` | Wall-clock microseconds of the operation |
| `rows_affected` | Documents inserted/modified/deleted/returned |
| `op_type` | 0=SELECT, 1=INSERT, 2=UPDATE, 3=DELETE |
| `error_code` | 0 = success, 1 = fault |

## Error Types

`MongoFault` is a `Copy` enum — no heap allocations in fault paths:

```rust
pub enum MongoFault {
    ConnectionFailed { uri_hash: u32, reason_code: u32 },
    QueryFailed     { collection_hash: u32, reason_code: u32 },
    InsertFailed    { collection_hash: u32, reason_code: u32 },
    UpdateFailed    { collection_hash: u32, reason_code: u32 },
    DeleteFailed    { collection_hash: u32, reason_code: u32 },
    Timeout         { collection_hash: u32, elapsed_ms: u32 },
    DeserializeFailed { collection_hash: u32 },
}
```

All string context is stored as `u32` FxHash values registered with
`vil_log::dict::register_str()` for reverse lookup in log drains.

## License

Apache-2.0 — see workspace root `LICENSE-APACHE`.
