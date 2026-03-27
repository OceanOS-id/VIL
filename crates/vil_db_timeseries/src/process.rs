// =============================================================================
// vil_db_timeseries — ServiceProcess registration helper
// =============================================================================
//
// Provides a convenience function to create a shared `TimeseriesClient` ready
// for use as a VIL service component.
//
// # Usage in a VilApp context
//
// ```ignore
// use vil_db_timeseries::process::create_client;
//
// let client = create_client(config).await?;
// ServiceProcess::new("timeseries")
//     .state(client)
//     .endpoint(...)
// ```

use std::sync::Arc;

use crate::{TimeseriesClient, TimeseriesConfig, TimeseriesFault};

/// Create a shared `TimeseriesClient` wrapped in an `Arc` for multi-owner access.
///
/// Connects to the configured time-series backend (InfluxDB or TimescaleDB)
/// using `config` and returns the client ready to be stored as `ServiceProcess`
/// state.
pub async fn create_client(
    config: TimeseriesConfig,
) -> Result<Arc<TimeseriesClient>, TimeseriesFault> {
    let client = TimeseriesClient::new(config).await?;
    Ok(Arc::new(client))
}
