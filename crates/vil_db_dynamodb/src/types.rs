// =============================================================================
// vil_db_dynamodb::types — Result type alias
// =============================================================================

use crate::error::DynamoFault;

/// Convenience Result type for all DynamoDB operations.
pub type DynamoResult<T> = Result<T, DynamoFault>;
