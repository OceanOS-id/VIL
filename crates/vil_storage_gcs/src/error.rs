// =============================================================================
// vil_storage_gcs::error — GcsFault
// =============================================================================
//
// Error type for GCS operations. Uses plain enum style following `#[vil_fault]`
// conventions: no heap strings, only u32 hashes and numeric codes.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for Google Cloud Storage operations.
///
/// All string data is represented as u32 FxHash values produced via
/// `vil_log::dict::register_str`. Resolve hashes using `vil_log::dict::lookup`.
#[connector_fault]
pub enum GcsFault {
    /// Could not establish a connection to the GCS endpoint.
    ConnectionFailed {
        /// FxHash of the endpoint or project label.
        endpoint_hash: u32,
        /// Low-level reason code.
        reason_code: u32,
    },
    /// The requested object was not found.
    NotFound {
        /// FxHash of the object name.
        name_hash: u32,
    },
    /// Credentials were rejected or the caller lacks permission.
    AccessDenied {
        /// FxHash of the object name that triggered the denial.
        name_hash: u32,
    },
    /// The configured bucket does not exist.
    BucketNotFound {
        /// FxHash of the bucket name.
        bucket_hash: u32,
    },
    /// An upload failed.
    UploadFailed {
        /// FxHash of the object name.
        name_hash: u32,
        /// Number of bytes that were attempted.
        size: u64,
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
