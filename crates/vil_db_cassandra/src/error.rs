// =============================================================================
// vil_db_cassandra::error — CassandraFault
// =============================================================================
//
// VIL-compliant fault enum for Cassandra/ScyllaDB operations.
// No String fields, no thiserror — only u32/u64 numeric codes.
// =============================================================================

/// Fault type for all Cassandra/ScyllaDB operations.
///
/// All string-derived context is stored as u32 FxHash values registered via
/// `vil_log::dict::register_str()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl CassandraFault {
    /// Return a stable numeric error code for log `error_code` fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            CassandraFault::ConnectionFailed { .. } => 1,
            CassandraFault::PrepareFailed { .. } => 2,
            CassandraFault::ExecuteFailed { .. } => 3,
            CassandraFault::QueryFailed { .. } => 4,
            CassandraFault::BatchFailed { .. } => 5,
            CassandraFault::PagedFailed { .. } => 6,
        }
    }
}
