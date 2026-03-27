// =============================================================================
// vil_storage_gcs::error — GcsFault
// =============================================================================
//
// Error type for GCS operations. Uses plain enum style following `#[vil_fault]`
// conventions: no heap strings, only u32 hashes and numeric codes.
// =============================================================================

/// Fault type for Google Cloud Storage operations.
///
/// All string data is represented as u32 FxHash values produced via
/// `vil_log::dict::register_str`. Resolve hashes using `vil_log::dict::lookup`.
#[derive(Debug, Clone, Copy)]
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

impl std::fmt::Display for GcsFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GcsFault::ConnectionFailed { endpoint_hash, reason_code } => {
                write!(f, "GCS connection failed (endpoint_hash={endpoint_hash}, reason={reason_code})")
            }
            GcsFault::NotFound { name_hash } => {
                write!(f, "GCS object not found (name_hash={name_hash})")
            }
            GcsFault::AccessDenied { name_hash } => {
                write!(f, "GCS access denied (name_hash={name_hash})")
            }
            GcsFault::BucketNotFound { bucket_hash } => {
                write!(f, "GCS bucket not found (bucket_hash={bucket_hash})")
            }
            GcsFault::UploadFailed { name_hash, size } => {
                write!(f, "GCS upload failed (name_hash={name_hash}, size={size})")
            }
            GcsFault::Timeout { operation_hash, elapsed_ms } => {
                write!(f, "GCS timeout (op_hash={operation_hash}, elapsed={elapsed_ms}ms)")
            }
            GcsFault::Unknown { message_hash } => {
                write!(f, "GCS unknown error (message_hash={message_hash})")
            }
        }
    }
}

impl std::error::Error for GcsFault {}
