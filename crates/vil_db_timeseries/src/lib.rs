// =============================================================================
// vil_db_timeseries — VIL Database Plugin: Time-Series (InfluxDB / TimescaleDB)
// =============================================================================
//
// Time-series client wrapper with VIL semantic log integration.
//
// # Backends
//
// | Feature      | Backend     | Driver       |
// |--------------|-------------|--------------|
// | `influxdb`   | InfluxDB v2 | influxdb2    |
// | `timescale`  | TimescaleDB | vil_db_sqlx  |
//
// TimescaleDB does NOT add a new driver; it reuses `vil_db_sqlx`.
// See `ops::TimeseriesClient::timescale_note` for guidance.
//
// # Compliance
//
// - COMPLIANCE.md §8  (Semantic Log):  `vil_log` dependency, `db_log!` on every
//   operation, no println!/tracing, `register_str()` for all hash fields.
// - COMPLIANCE.md §11 (Crate Structure): config / client / ops / error / types
//   module layout.
//
// # Quick Start (InfluxDB)
//
// ```rust,no_run
// # #[cfg(feature = "influxdb")]
// use vil_db_timeseries::{TimeseriesClient, TimeseriesConfig};
//
// # #[cfg(feature = "influxdb")]
// #[tokio::main]
// async fn main() {
//     let config = TimeseriesConfig::new(
//         "http://localhost:8086", "myorg", "my-token", "metrics",
//     );
//     let client = TimeseriesClient::new(config).await.expect("connect");
//
//     // query_flux returns raw CSV
//     let csv = client.query_flux(
//         r#"from(bucket:"metrics") |> range(start: -1h)"#
//     ).await.unwrap();
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
pub use client::TimeseriesClient;
pub use config::TimeseriesConfig;
pub use error::TimeseriesFault;
pub use events::TimeseriesPointsWritten;
pub use state::TimeseriesClientState;
pub use types::TimeseriesResult;
