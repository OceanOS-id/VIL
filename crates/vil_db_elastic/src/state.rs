// =============================================================================
// vil_db_elastic::state — Elasticsearch client health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the Elasticsearch client, reported via ServiceProcess.
#[connector_state]
pub struct ElasticClientState {
    pub total_indexes: u64,
    pub total_searches: u64,
    pub total_errors: u64,
    pub avg_latency_us: u32,
}
