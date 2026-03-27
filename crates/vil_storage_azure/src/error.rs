// =============================================================================
// vil_storage_azure::error — AzureFault
// =============================================================================
//
// Error type for Azure Blob Storage operations. Uses plain enum style
// following `#[vil_fault]` conventions: no heap strings, only u32 hashes
// and numeric codes.
// =============================================================================

/// Fault type for Azure Blob Storage operations.
///
/// All string data is represented as u32 FxHash values produced via
/// `vil_log::dict::register_str`. Resolve hashes using `vil_log::dict::lookup`.
#[derive(Debug, Clone, Copy)]
pub enum AzureFault {
    /// Could not establish a connection to the Azure storage endpoint.
    ConnectionFailed {
        /// FxHash of the account name.
        account_hash: u32,
        /// Low-level reason code.
        reason_code: u32,
    },
    /// The requested blob was not found.
    NotFound {
        /// FxHash of the blob name.
        name_hash: u32,
    },
    /// Credentials were rejected or the caller lacks permission.
    AccessDenied {
        /// FxHash of the blob name that triggered the denial.
        name_hash: u32,
    },
    /// The configured container does not exist.
    ContainerNotFound {
        /// FxHash of the container name.
        container_hash: u32,
    },
    /// An upload failed.
    UploadFailed {
        /// FxHash of the blob name.
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

impl std::fmt::Display for AzureFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AzureFault::ConnectionFailed { account_hash, reason_code } => {
                write!(f, "Azure connection failed (account_hash={account_hash}, reason={reason_code})")
            }
            AzureFault::NotFound { name_hash } => {
                write!(f, "Azure blob not found (name_hash={name_hash})")
            }
            AzureFault::AccessDenied { name_hash } => {
                write!(f, "Azure access denied (name_hash={name_hash})")
            }
            AzureFault::ContainerNotFound { container_hash } => {
                write!(f, "Azure container not found (container_hash={container_hash})")
            }
            AzureFault::UploadFailed { name_hash, size } => {
                write!(f, "Azure upload failed (name_hash={name_hash}, size={size})")
            }
            AzureFault::Timeout { operation_hash, elapsed_ms } => {
                write!(f, "Azure timeout (op_hash={operation_hash}, elapsed={elapsed_ms}ms)")
            }
            AzureFault::Unknown { message_hash } => {
                write!(f, "Azure unknown error (message_hash={message_hash})")
            }
        }
    }
}

impl std::error::Error for AzureFault {}
