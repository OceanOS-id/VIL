// =============================================================================
// vil_db_cassandra::error — CassandraFault
// =============================================================================
//
// VIL-compliant fault enum for Cassandra/ScyllaDB operations.
// No String fields, no thiserror — only u32/u64 numeric codes.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for all Cassandra/ScyllaDB operations.
///
/// All string-derived context is stored as u32 FxHash values registered via
/// `vil_log::dict::register_str()`.
#[connector_fault]
pub enum CassandraFault {
    /// Failed to connect to the cluster.
    ConnectionFailed {
        /// FxHash of the contact-point URI.
        uri_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// Failed to prepare a statement.
    PrepareFailed {
        /// FxHash of the query template.
        query_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// Execution of a prepared statement failed.
    ExecuteFailed {
        /// FxHash of the query template.
        query_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// Ad-hoc query execution failed.
    QueryFailed {
        /// FxHash of the query string.
        query_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// Batch execution failed.
    BatchFailed {
        /// Numeric reason code.
        reason_code: u32,
    },
    /// Paged execution failed.
    PagedFailed {
        /// FxHash of the query template.
        query_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
}
