// =============================================================================
// vil_db_cassandra::events — Cassandra connector events
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a query is successfully executed on Cassandra/ScyllaDB.
#[connector_event]
pub struct CassandraQueryExecuted {
    pub keyspace_hash: u32,
    pub query_hash: u32,
    pub elapsed_us: u32,
    pub timestamp_ns: u64,
}
