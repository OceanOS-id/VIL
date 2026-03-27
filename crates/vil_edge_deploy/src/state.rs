// =============================================================================
// vil_edge_deploy::state — connector state for ServiceProcess health metrics
// =============================================================================

use vil_connector_macros::connector_state;

/// Live state metrics for the edge deployment process.
#[connector_state]
pub struct EdgeDeployState {
    /// Total edge builds completed successfully.
    pub builds_completed: u64,
    /// Total build failures.
    pub build_failures: u64,
    /// FxHash of the most recent target binary hash (0 if none built).
    pub target_hash: u32,
    /// Timestamp (ns) of the most recent successful build (0 if none).
    pub last_build_ns: u64,
}
