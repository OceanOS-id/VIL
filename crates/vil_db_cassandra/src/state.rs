// =============================================================================
// vil_db_cassandra::state — Cassandra connection pool health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the Cassandra/ScyllaDB session pool, reported via ServiceProcess.
#[connector_state]
pub struct CassandraPoolState {
    pub active_connections: u32,
    pub idle_connections: u32,
    pub total_queries: u64,
    pub total_errors: u64,
}
