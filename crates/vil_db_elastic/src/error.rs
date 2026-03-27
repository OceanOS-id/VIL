// =============================================================================
// vil_db_elastic::error — ElasticFault
// =============================================================================
//
// Error type for Elasticsearch operations. Uses plain enum style following
// `#[vil_fault]` conventions: no heap strings, only u32 hashes and numeric codes.
// =============================================================================

/// Fault type for Elasticsearch / OpenSearch operations.
///
/// All string data is represented as u32 FxHash values produced via
/// `vil_log::dict::register_str`. Resolve hashes using `vil_log::dict::lookup`.
#[derive(Debug, Clone, Copy)]
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

impl std::fmt::Display for ElasticFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElasticFault::ConnectionFailed { url_hash, reason_code } => {
                write!(f, "Elastic connection failed (url_hash={url_hash}, reason={reason_code})")
            }
            ElasticFault::NotFound { index_hash, id_hash } => {
                write!(f, "Elastic document not found (index_hash={index_hash}, id_hash={id_hash})")
            }
            ElasticFault::IndexNotFound { index_hash } => {
                write!(f, "Elastic index not found (index_hash={index_hash})")
            }
            ElasticFault::AccessDenied { index_hash } => {
                write!(f, "Elastic access denied (index_hash={index_hash})")
            }
            ElasticFault::IndexFailed { index_hash, id_hash } => {
                write!(f, "Elastic index failed (index_hash={index_hash}, id_hash={id_hash})")
            }
            ElasticFault::SearchFailed { index_hash, query_hash } => {
                write!(f, "Elastic search failed (index_hash={index_hash}, query_hash={query_hash})")
            }
            ElasticFault::BulkFailed { index_hash, failed_count } => {
                write!(f, "Elastic bulk failed (index_hash={index_hash}, failed={failed_count})")
            }
            ElasticFault::Timeout { operation_hash, elapsed_ms } => {
                write!(f, "Elastic timeout (op_hash={operation_hash}, elapsed={elapsed_ms}ms)")
            }
            ElasticFault::Unknown { message_hash } => {
                write!(f, "Elastic unknown error (message_hash={message_hash})")
            }
        }
    }
}

impl std::error::Error for ElasticFault {}
