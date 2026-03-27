// =============================================================================
// vil_db_elastic::error — ElasticFault
// =============================================================================
//
// Error type for Elasticsearch operations. Uses plain enum style following
// `#[vil_fault]` conventions: no heap strings, only u32 hashes and numeric codes.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for Elasticsearch / OpenSearch operations.
///
/// All string data is represented as u32 FxHash values produced via
/// `vil_log::dict::register_str`. Resolve hashes using `vil_log::dict::lookup`.
#[connector_fault]
pub enum ElasticFault {
    /// Could not establish a connection to the Elasticsearch node.
    ConnectionFailed {
        /// FxHash of the node URL.
        url_hash: u32,
        /// Low-level reason code.
        reason_code: u32,
    },
    /// The requested document was not found.
    NotFound {
        /// FxHash of the index name.
        index_hash: u32,
        /// FxHash of the document ID.
        id_hash: u32,
    },
    /// The requested index does not exist.
    IndexNotFound {
        /// FxHash of the index name.
        index_hash: u32,
    },
    /// Credentials were rejected or the caller lacks permission.
    AccessDenied {
        /// FxHash of the index name.
        index_hash: u32,
    },
    /// An indexing (insert/update) operation failed.
    IndexFailed {
        /// FxHash of the index name.
        index_hash: u32,
        /// FxHash of the document ID.
        id_hash: u32,
    },
    /// A search query failed.
    SearchFailed {
        /// FxHash of the index name.
        index_hash: u32,
        /// FxHash of the query.
        query_hash: u32,
    },
    /// A bulk operation failed (partial or complete).
    BulkFailed {
        /// FxHash of the index name.
        index_hash: u32,
        /// Number of failed items in the bulk request.
        failed_count: u32,
    },
    /// An operation exceeded its time budget.
    Timeout {
        /// FxHash of the operation name.
        operation_hash: u32,
        /// Elapsed milliseconds.
        elapsed_ms: u32,
    },
    /// An unexpected / unclassified error.
    Unknown {
        /// FxHash of the error message string.
        message_hash: u32,
    },
}
