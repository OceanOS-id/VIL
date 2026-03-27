// =============================================================================
// vil_otel::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the OTel export bridge.
#[connector_state]
pub struct OtelExportState {
    /// Total spans exported successfully.
    pub spans_exported: u64,
    /// Total metrics exported successfully.
    pub metrics_exported: u64,
    /// Total export errors encountered.
    pub export_errors: u64,
    /// Timestamp (ns) of the most recent successful export (0 if none).
    pub last_export_ns: u64,
}
