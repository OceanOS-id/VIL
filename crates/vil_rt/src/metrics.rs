// =============================================================================
// vil_rt::metrics — Runtime Metrics Snapshot
// =============================================================================
// Struct for runtime metrics snapshots. Used for observability,
// debugging, and reporting.
//
// TASK LIST:
// [x] RuntimeMetrics struct
// =============================================================================

/// Snapshot of runtime metrics at a point in time.
#[derive(Clone, Copy, Debug, Default)]
pub struct RuntimeMetrics {
    /// Total depth of all queues (number of pending descriptors).
    pub queue_depth_total: usize,
    /// Number of samples still active in the registry.
    pub in_flight_samples: usize,
    /// Number of registered processes.
    pub registered_processes: usize,
}
