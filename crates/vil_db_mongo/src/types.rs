// =============================================================================
// vil_db_mongo::types — Result type aliases
// =============================================================================

use crate::error::MongoFault;

/// Convenience Result type for all MongoDB operations.
pub type MongoResult<T> = Result<T, MongoFault>;
