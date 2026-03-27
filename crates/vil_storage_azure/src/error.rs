// =============================================================================
// vil_storage_azure::error — AzureFault
// =============================================================================
//
// Error type for Azure Blob Storage operations. Uses plain enum style
// following `#[vil_fault]` conventions: no heap strings, only u32 hashes
// and numeric codes.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for Azure Blob Storage operations.
///
/// All string data is represented as u32 FxHash values produced via
/// `vil_log::dict::register_str`. Resolve hashes using `vil_log::dict::lookup`.
#[connector_fault]
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
