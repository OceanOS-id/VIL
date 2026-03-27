// =============================================================================
// vil_db_mongo — VIL Database Plugin: MongoDB
// =============================================================================
//
// MongoDB client wrapper with VIL semantic log integration.
//
// # Compliance
//
// - COMPLIANCE.md §8  (Semantic Log):  `vil_log` dependency, `db_log!` on every
//   operation, no println!/tracing, `register_str()` for all hash fields.
// - COMPLIANCE.md §11 (Crate Structure): config / client / crud / error / types
//   module layout.
//
// # Boundary Classification (COMPLIANCE.md §2)
//
// | Path                        | Mode  | Notes                                  |
// |-----------------------------|-------|----------------------------------------|
// | Network I/O (MongoDB wire)  | Copy  | Required by BSON/TCP protocol          |
// | Internal pipeline path      | Copy  | Phase 0 — SHM bridge deferred          |
// | Config/metadata             | Copy  | Setup-time, not hot-path               |
//
// # Tri-Lane Mapping (COMPLIANCE.md §5)
//
// | Lane    | Direction       | Content                           |
// |---------|-----------------|-----------------------------------|
// | Trigger | Inbound → VIL   | Query request descriptor          |
// | Data    | Outbound ← VIL  | Query result set                  |
// | Control | Bidirectional   | Connection error / tx commit/roll |
//
// # Thread hint for LogConfig
//
// This crate delegates all async I/O to the mongodb driver's internal pool.
// The driver manages its own connection threads. Add `max_pool` to your
// `LogConfig.threads` estimate for optimal ring sizing.
//
// # Quick Start
//
// ```rust,no_run
// use vil_db_mongo::{MongoClient, MongoConfig};
// use bson::doc;
// use serde::{Deserialize, Serialize};
//
// #[derive(Debug, Serialize, Deserialize)]
// struct User { name: String, age: u32 }
//
// #[tokio::main]
// async fn main() {
//     let config = MongoConfig::new("mongodb://localhost:27017", "myapp");
//     let client = MongoClient::new(config).await.expect("connect");
//
//     let id = client.insert_one("users", &User { name: "Alice".into(), age: 30 }).await.unwrap();
//     let user: Option<User> = client.find_one("users", doc! { "_id": id }).await.unwrap();
// }
// ```
// =============================================================================

pub mod client;
pub mod config;
pub mod crud;
pub mod error;
pub mod events;
pub mod process;
pub mod state;
pub mod types;

// Flat re-exports
pub use client::MongoClient;
pub use config::MongoConfig;
pub use error::MongoFault;
pub use events::{MongoDocumentDeleted, MongoDocumentInserted, MongoDocumentUpdated};
pub use state::MongoPoolState;
pub use types::MongoResult;
