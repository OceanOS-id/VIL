// =============================================================================
// vil_db_timeseries::events — Time-series connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when data points are successfully written to the time-series backend.
#[connector_event]
pub struct TimeseriesPointsWritten {
    pub bucket_hash: u32,
    pub points_count: u32,
    pub elapsed_ns: u64,
    pub timestamp_ns: u64,
}
