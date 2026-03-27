// =============================================================================
// vil_db_mongo::state — MongoDB connection pool health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the MongoDB connection pool, reported via ServiceProcess.
#[connector_state]
pub struct MongoPoolState {
    pub active_connections: u32,
    pub idle_connections: u32,
    pub waiting_requests: u32,
    pub total_queries: u64,
}
