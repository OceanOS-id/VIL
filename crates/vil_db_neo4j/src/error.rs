// =============================================================================
// vil_db_neo4j::error — Neo4jFault
// =============================================================================
//
// VIL-compliant fault enum for Neo4j graph operations.
// No String fields, no thiserror — only u32/u64 numeric codes.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for all Neo4j operations.
///
/// All string-derived context is stored as u32 FxHash values registered via
/// `vil_log::dict::register_str()`.
#[connector_fault]
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
