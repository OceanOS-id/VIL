// =============================================================================
// vil_trigger_webhook::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a webhook request is successfully received and verified.
#[connector_event]
pub struct WebhookReceived {
    /// FxHash of the HTTP path string.
    pub path_hash: u32,
    /// HTTP method code: 0=GET, 1=POST, 2=PUT, 3=PATCH, 4=DELETE, 255=other.
    pub method_code: u8,
    /// Request body size in bytes.
    pub body_bytes: u32,
    /// Whether HMAC signature was present and valid (1=yes, 0=no-secret).
    pub signature_valid: u8,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
