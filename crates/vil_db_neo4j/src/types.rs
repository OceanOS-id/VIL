// =============================================================================
// vil_db_neo4j::types — Result type alias
// =============================================================================

use crate::error::Neo4jFault;

/// Convenience Result type for all Neo4j operations.
pub type Neo4jResult<T> = Result<T, Neo4jFault>;
