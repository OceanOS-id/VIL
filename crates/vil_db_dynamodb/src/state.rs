// =============================================================================
// vil_db_dynamodb::state — DynamoDB client health/metrics state
// =============================================================================

use vil_connector_macros::connector_state;

/// Live metrics for the DynamoDB client, reported via ServiceProcess.
#[connector_state]
pub struct DynamoClientState {
    pub total_puts: u64,
    pub total_gets: u64,
    pub total_deletes: u64,
    pub total_errors: u64,
    pub avg_latency_us: u32,
}
