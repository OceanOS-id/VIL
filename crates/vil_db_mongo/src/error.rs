// =============================================================================
// vil_db_mongo::error — MongoFault
// =============================================================================
//
// VIL-compliant fault enum for MongoDB operations.
// Uses u32 hashes for all string fields — no heap allocations in fault types.
// Complies with COMPLIANCE.md §4 (Semantic Type Compliance): no thiserror,
// no String fields.
// =============================================================================

/// Fault type for all MongoDB operations.
///
/// All string-derived context (URI, collection names) is stored as u32 FxHash
/// values registered via `vil_log::dict::register_str()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MongoFault {
    /// Failed to establish a connection to MongoDB.
    ConnectionFailed {
        /// FxHash of the URI string.
        uri_hash: u32,
        /// Numeric reason code from the driver error.
        reason_code: u32,
    },
    /// A query (find/count) operation failed.
    QueryFailed {
        /// FxHash of the collection name.
        collection_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// An insert operation failed.
    InsertFailed {
        /// FxHash of the collection name.
        collection_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// An update operation failed.
    UpdateFailed {
        /// FxHash of the collection name.
        collection_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// A delete operation failed.
    DeleteFailed {
        /// FxHash of the collection name.
        collection_hash: u32,
        /// Numeric reason code.
        reason_code: u32,
    },
    /// Operation exceeded the timeout threshold.
    Timeout {
        /// FxHash of the collection name.
        collection_hash: u32,
        /// Elapsed time in milliseconds.
        elapsed_ms: u32,
    },
    /// BSON deserialization into the target type failed.
    DeserializeFailed {
        /// FxHash of the collection name.
        collection_hash: u32,
    },
}

impl MongoFault {
    /// Return a stable numeric error code suitable for log `error_code` fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            MongoFault::ConnectionFailed { .. } => 1,
            MongoFault::QueryFailed { .. } => 2,
            MongoFault::InsertFailed { .. } => 3,
            MongoFault::UpdateFailed { .. } => 4,
            MongoFault::DeleteFailed { .. } => 5,
            MongoFault::Timeout { .. } => 6,
            MongoFault::DeserializeFailed { .. } => 7,
        }
    }
}
