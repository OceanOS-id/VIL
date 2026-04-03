// =============================================================================
// vil_soap::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when a SOAP action call completes successfully.
#[connector_event]
pub struct ActionCalled {
    /// FxHash of the SOAP action string.
    pub action_hash: u32,
    /// FxHash of the endpoint URL.
    pub endpoint_hash: u32,
    /// Round-trip latency in microseconds.
    pub latency_ns: u32,
    /// Response payload size in bytes.
    pub response_bytes: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
