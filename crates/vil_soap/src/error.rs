// =============================================================================
// vil_soap::error — SoapFault
// =============================================================================
//
// VIL-compliant fault type for SOAP operations.
// No thiserror, no String fields — COMPLIANCE.md §4 (Semantic Type Compliance).
// All string-derived context is stored as u32 FxHash via register_str().
// =============================================================================

use vil_connector_macros::connector_fault;

/// Fault type for all SOAP/WSDL client operations.
///
/// All string-derived fields (action name, endpoint, etc.) are stored as u32
/// FxHash values registered via `vil_log::dict::register_str()`.
#[connector_fault]
pub enum SoapFault {
    /// Failed to build the HTTP connection or TLS handshake.
    ConnectionFailed {
        /// FxHash of the endpoint URL.
        endpoint_hash: u32,
        /// Numeric driver error code.
        reason_code: u32,
    },
    /// The HTTP response returned a non-2xx status.
    HttpError {
        /// FxHash of the SOAP action string.
        action_hash: u32,
        /// HTTP status code as a u32.
        status_code: u32,
    },
    /// The server returned a SOAP Fault element.
    SoapFaultResponse {
        /// FxHash of the faultcode string.
        faultcode_hash: u32,
        /// FxHash of the faultstring.
        faultstring_hash: u32,
    },
    /// Failed to build the SOAP envelope (XML serialization error).
    EnvelopeBuildFailed {
        /// FxHash of the action that was being built.
        action_hash: u32,
    },
    /// Failed to parse the SOAP response envelope.
    EnvelopeParseFailed {
        /// FxHash of the action that was being called.
        action_hash: u32,
        /// Numeric XML parse error code.
        reason_code: u32,
    },
    /// The request exceeded the configured timeout.
    Timeout {
        /// FxHash of the SOAP action.
        action_hash: u32,
        /// Elapsed time in milliseconds.
        elapsed_ms: u32,
    },
}

impl SoapFault {
    /// Return a stable numeric error code for log `error_code` fields.
    pub fn as_error_code(&self) -> u8 {
        match self {
            SoapFault::ConnectionFailed { .. } => 1,
            SoapFault::HttpError { .. } => 2,
            SoapFault::SoapFaultResponse { .. } => 3,
            SoapFault::EnvelopeBuildFailed { .. } => 4,
            SoapFault::EnvelopeParseFailed { .. } => 5,
            SoapFault::Timeout { .. } => 6,
        }
    }
}
