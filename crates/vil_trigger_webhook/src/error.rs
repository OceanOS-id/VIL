// =============================================================================
// vil_trigger_webhook::error — WebhookFault
// =============================================================================
//
// VIL-compliant fault for webhook trigger operations.
// No thiserror, no String fields — COMPLIANCE.md §4.
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for all webhook trigger operations.
#[connector_fault]
pub enum WebhookFault {
    /// Failed to bind the HTTP listener socket.
    BindFailed {
        /// FxHash of the listen address string.
        addr_hash: u32,
        /// OS error code.
        os_code: u32,
    },
    /// HMAC signature verification failed for a received request.
    SignatureInvalid {
        /// FxHash of the request path.
        path_hash: u32,
    },
    /// The `X-Hub-Signature-256` header is missing from the request.
    MissingSignatureHeader {
        /// FxHash of the request path.
        path_hash: u32,
    },
    /// The request body could not be read.
    BodyReadFailed {
        /// Content-length hint (0 if unknown).
        content_length: u32,
    },
    /// The HTTP router shut down unexpectedly.
    ServerShutdown {
        /// FxHash of the listen address.
        addr_hash: u32,
    },
}

impl WebhookFault {
    /// Return a stable numeric code for log fields.
    pub fn as_error_code(&self) -> u32 {
        match self {
            WebhookFault::BindFailed { .. }              => 1,
            WebhookFault::SignatureInvalid { .. }        => 2,
            WebhookFault::MissingSignatureHeader { .. }  => 3,
            WebhookFault::BodyReadFailed { .. }          => 4,
            WebhookFault::ServerShutdown { .. }          => 5,
        }
    }
}
