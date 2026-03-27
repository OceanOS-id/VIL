// =============================================================================
// vil_db_clickhouse — VIL ClickHouse OLAP Plugin
// =============================================================================
//
// ClickHouse client for OLAP analytics with batch INSERT and streaming queries.
//
// # Compliance
//
// - Section 8  (Semantic Log): `vil_log` dependency, `db_log!` auto-emit on
//   every query/insert, `register_str()` for all string fields.
// - Section 11 (Crate Structure): standard module layout.
// - No `println!`, `eprintln!`, `tracing::info!`, or `log::*` calls.
//
// # Quick start
//
// ```rust,no_run
// use vil_db_clickhouse::{ChClient, ClickHouseConfig};
//
// #[tokio::main]
// async fn main() {
//     let client = ChClient::new(ClickHouseConfig::default());
//     client.execute("CREATE TABLE IF NOT EXISTS hits (ts UInt64) ENGINE=Null").await.ok();
// }
// ```

pub mod batch;
pub mod client;
pub mod config;
pub mod error;
pub mod events;
pub mod process;
pub mod state;

// ---------------------------------------------------------------------------
// Flat re-exports
// ---------------------------------------------------------------------------

pub use batch::BatchInserter;
pub use client::ChClient;
pub use config::ClickHouseConfig;
pub use error::ChFault;
pub use events::ChBatchInserted;
pub use state::ChClientState;
