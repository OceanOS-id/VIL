// =============================================================================
// vil_trigger_email::error — EmailFault
// =============================================================================
//
// VIL-compliant fault for IMAP trigger operations.
// No thiserror, no String fields — COMPLIANCE.md §4.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for all IMAP email trigger operations.
#[connector_fault]
pub enum EmailFault {
    /// TLS connection to the IMAP server failed.
    TlsConnectFailed {
        /// FxHash of "host:port".
        host_hash: u32,
        /// OS error code.
        reason_code: u32,
    },
    /// IMAP login was rejected (bad credentials).
    LoginFailed {
        /// FxHash of the username.
        user_hash: u32,
    },
    /// The requested mailbox folder was not found.
    FolderNotFound {
        /// FxHash of the folder name.
        folder_hash: u32,
    },
    /// IDLE command failed or is not supported.
    IdleFailed {
        /// FxHash of "host:port".
        host_hash: u32,
        /// Error code.
        reason_code: u32,
    },
    /// Unexpected disconnection from the IMAP server.
    Disconnected {
        /// FxHash of "host:port".
        host_hash: u32,
    },
    /// FETCH of the new message failed.
    FetchFailed {
        /// IMAP sequence number of the failed message.
        seq: u32,
    },
}

impl EmailFault {
    /// Return a stable numeric code for log fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            EmailFault::TlsConnectFailed { .. } => 1,
            EmailFault::LoginFailed { .. } => 2,
            EmailFault::FolderNotFound { .. } => 3,
            EmailFault::IdleFailed { .. } => 4,
            EmailFault::Disconnected { .. } => 5,
            EmailFault::FetchFailed { .. } => 6,
        }
    }
}
