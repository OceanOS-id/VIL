// =============================================================================
// vil_db_neo4j::error — Neo4jFault
// =============================================================================
//
// VIL-compliant fault enum for Neo4j graph operations.
// No String fields, no thiserror — only u32/u64 numeric codes.
// =============================================================================

/// Fault type for all Neo4j operations.
///
/// All string-derived context is stored as u32 FxHash values registered via
/// `vil_log::dict::register_str()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neo4jFault {
    /// Failed to connect to the Neo4j instance.
    ConnectionFailed {
        /// FxHash of the URI string.
        uri_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// Cypher query execution failed.
    ExecuteFailed {
        /// FxHash of the Cypher query.
        query_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// Transaction failed or was rolled back.
    TransactionFailed {
        /// Numeric reason code.
        reason_code: u32,
    },
    /// CREATE node operation failed.
    CreateNodeFailed {
        /// FxHash of the label string.
        label_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// MATCH query failed.
    MatchFailed {
        /// FxHash of the Cypher query.
        query_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
}

impl Neo4jFault {
    /// Return a stable numeric error code for log `error_code` fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            Neo4jFault::ConnectionFailed { .. } => 1,
            Neo4jFault::ExecuteFailed { .. } => 2,
            Neo4jFault::TransactionFailed { .. } => 3,
            Neo4jFault::CreateNodeFailed { .. } => 4,
            Neo4jFault::MatchFailed { .. } => 5,
        }
    }
}
