// =============================================================================
// vil_storage_azure::state — Azure client health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the Azure Blob Storage client, reported via ServiceProcess.
#[connector_state]
pub struct AzureClientState {
    pub total_puts: u64,
    pub total_gets: u64,
    pub total_errors: u64,
    pub avg_latency_ns: u32,
}
