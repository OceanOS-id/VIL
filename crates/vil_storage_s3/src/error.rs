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

/// Fault type for S3-compatible storage operations.
///
/// All string data (endpoint, key, bucket) is represented as u32 FxHash values
/// produced via `vil_log::dict::register_str`. Resolve hashes for display using
/// `vil_log::dict::lookup`.
#[derive(Debug, Clone, Copy)]
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

impl std::fmt::Display for S3Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            S3Fault::ConnectionFailed { endpoint_hash, reason_code } => {
                write!(f, "S3 connection failed (endpoint_hash={endpoint_hash}, reason={reason_code})")
            }
            S3Fault::NotFound { key_hash } => {
                write!(f, "S3 object not found (key_hash={key_hash})")
            }
            S3Fault::AccessDenied { key_hash } => {
                write!(f, "S3 access denied (key_hash={key_hash})")
            }
            S3Fault::BucketNotFound { bucket_hash } => {
                write!(f, "S3 bucket not found (bucket_hash={bucket_hash})")
            }
            S3Fault::UploadFailed { key_hash, size } => {
                write!(f, "S3 upload failed (key_hash={key_hash}, size={size})")
            }
            S3Fault::Timeout { operation_hash, elapsed_ms } => {
                write!(f, "S3 timeout (op_hash={operation_hash}, elapsed={elapsed_ms}ms)")
            }
            S3Fault::Unknown { message_hash } => {
                write!(f, "S3 unknown error (message_hash={message_hash})")
            }
        }
    }
}

impl std::error::Error for S3Fault {}
