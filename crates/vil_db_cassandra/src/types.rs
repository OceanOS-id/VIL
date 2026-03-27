// =============================================================================
// vil_db_cassandra::types — Result type alias
// =============================================================================

use crate::error::CassandraFault;

/// Convenience Result type for all Cassandra/ScyllaDB operations.
pub type CassandraResult<T> = Result<T, CassandraFault>;
