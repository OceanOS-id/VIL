// =============================================================================
// vil_soap::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the SOAP client.
#[connector_state]
pub struct SoapClientState {
    /// Total SOAP actions called successfully.
    pub actions_called: u64,
    /// Total SOAP faults received from server.
    pub soap_faults: u64,
    /// Total HTTP/transport errors.
    pub transport_errors: u64,
    /// Average round-trip latency in microseconds.
    pub avg_latency_ns: u32,
}
