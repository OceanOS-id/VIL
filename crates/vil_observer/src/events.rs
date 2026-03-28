// =============================================================================
// vil_observer::events — connector events emitted on Data Lane
// =============================================================================

use vil_connector_macros::connector_event;

/// Emitted when the observer captures a periodic metrics snapshot.
#[connector_event]
pub struct ObserverMetricsSnapshot {
    /// Total requests across all endpoints since startup.
    pub total_requests: u64,
    /// Number of registered endpoints.
    pub endpoint_count: u32,
    /// Server uptime in seconds.
    pub uptime_secs: u64,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when the observer dashboard is accessed.
#[connector_event]
pub struct ObserverDashboardAccess {
    /// FxHash of the client IP string.
    pub client_hash: u32,
    /// FxHash of the requested path.
    pub path_hash: u32,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}

/// Emitted when the observer detects an endpoint crossing an error-rate threshold.
#[connector_event]
pub struct ObserverErrorAlert {
    /// FxHash of the endpoint path.
    pub endpoint_hash: u32,
    /// Current error rate (0–10000, representing 0.00%–100.00%).
    pub error_rate_bps: u32,
    /// Total requests on this endpoint.
    pub request_count: u64,
    /// Wall-clock timestamp in nanoseconds (UNIX_EPOCH).
    pub timestamp_ns: u64,
}
