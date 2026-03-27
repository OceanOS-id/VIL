// =============================================================================
// vil_db_timeseries::state — Time-series client health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the time-series client, reported via ServiceProcess.
#[connector_state]
pub struct TimeseriesClientState {
    pub total_writes: u64,
    pub total_queries: u64,
    pub total_errors: u64,
    pub avg_latency_us: u32,
}
