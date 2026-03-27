// =============================================================================
// vil_db_clickhouse::state — ClickHouse client health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the ClickHouse client, reported via ServiceProcess.
#[connector_state]
pub struct ChClientState {
    pub total_inserts: u64,
    pub total_queries: u64,
    pub total_errors: u64,
    pub avg_batch_size: u32,
}
