// =============================================================================
// vil_storage_s3::error — S3Fault
// =============================================================================
//
// Error type for S3 operations. Uses plain enum style following `#[vil_fault]`
// conventions: no heap strings, only u32 hashes and numeric codes.
//
// All fields use `register_str()` hashes instead of raw `String` values so
// the error type stays copy-friendly and free of heap allocation on the
// hot path.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for S3-compatible storage operations.
///
/// All string data (endpoint, key, bucket) is represented as u32 FxHash values
/// produced via `vil_log::dict::register_str`. Resolve hashes for display using
/// `vil_log::dict::lookup`.
#[connector_fault]
pub enum S3Fault {
    /// Could not establish a connection to the S3 endpoint.
    ConnectionFailed {
        /// FxHash of the endpoint URL.
        endpoint_hash: u32,
        /// Low-level reason code (OS errno, HTTP status, etc.).
        reason_code: u32,
    },
    /// The requested object key was not found.
    NotFound {
        /// FxHash of the object key.
        key_hash: u32,
    },
    /// Credentials were rejected or the caller lacks permission.
    AccessDenied {
        /// FxHash of the object key that triggered the denial.
        key_hash: u32,
    },
    /// The configured bucket does not exist.
    BucketNotFound {
        /// FxHash of the bucket name.
        bucket_hash: u32,
    },
    /// An upload (put_object) failed.
    UploadFailed {
        /// FxHash of the object key.
        key_hash: u32,
        /// Number of bytes that were attempted.
        size: u64,
    },
    /// An operation exceeded its time budget.
    Timeout {
        /// FxHash of the operation name (e.g. "put_object").
        operation_hash: u32,
        /// How long the operation ran before timing out, in milliseconds.
        elapsed_ms: u32,
    },
    /// An unexpected / unclassified error.
    Unknown {
        /// FxHash of the error message string.
        message_hash: u32,
    },
}
