// =============================================================================
// vil_storage_s3::state — S3 client health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the S3 client, reported via ServiceProcess.
#[connector_state]
pub struct S3ClientState {
    pub total_puts: u64,
    pub total_gets: u64,
    pub total_errors: u64,
    pub avg_latency_ns: u32,
}
