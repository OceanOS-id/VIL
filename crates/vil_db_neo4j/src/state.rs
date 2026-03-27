// =============================================================================
// vil_db_neo4j::state — Neo4j session health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the Neo4j session, reported via ServiceProcess.
#[connector_state]
pub struct Neo4jSessionState {
    pub active_sessions: u32,
    pub total_queries: u64,
    pub total_transactions: u64,
    pub total_errors: u64,
}
