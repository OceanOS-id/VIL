// =============================================================================
// vil_db_cassandra — VIL Database Plugin: Apache Cassandra / ScyllaDB
// =============================================================================
//
// ScyllaDB/Cassandra session wrapper with VIL semantic log integration.
//
// # Compliance
//
// - COMPLIANCE.md §8  (Semantic Log):  `vil_log` dependency, `db_log!` on every
//   operation, no println!/tracing, `register_str()` for all hash fields.
// - COMPLIANCE.md §11 (Crate Structure): config / client / ops / error / types
//   module layout.
//
// # Quick Start
//
// ```rust,no_run
// use vil_db_cassandra::{CassandraClient, CassandraConfig};
//
// #[tokio::main]
// async fn main() {
//     let config = CassandraConfig::new("127.0.0.1:9042", "myapp");
//     let client = CassandraClient::new(config).await.expect("connect");
//
//     let result = client.query("SELECT * FROM users", &[]).await.unwrap();
// }
// ```
// =============================================================================

pub mod client;
pub mod config;
pub mod error;
pub mod events;
pub mod ops;
pub mod process;
pub mod state;
pub mod types;

// Flat re-exports
pub use client::CassandraClient;
pub use config::CassandraConfig;
pub use error::CassandraFault;
pub use events::CassandraQueryExecuted;
pub use state::CassandraPoolState;
pub use types::CassandraResult;
