// =============================================================================
// vil_storage_gcs::state — GCS client health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the GCS client, reported via ServiceProcess.
#[connector_state]
pub struct GcsClientState {
    pub total_puts: u64,
    pub total_gets: u64,
    pub total_errors: u64,
    pub avg_latency_us: u32,
}
