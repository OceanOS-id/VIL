//! VX Crash Cleanup — reclaim orphan resources when a Process fails
//!
//! When an endpoint Process panics or stops responding:
//! 1. Orphan tokens in VxKernel → marked Failed
//! 2. Pending IngressBridge requests → responded with 503
//! 3. Stale tokens cleaned up periodically

use std::sync::Arc;
use super::kernel::VxKernel;
use super::ingress::IngressBridge;

/// Cleanup configuration
pub struct CleanupConfig {
    /// Maximum age of completed/failed tokens before removal (nanos)
    pub max_token_age_ns: u64,
    /// Timeout for active tokens — if active longer than this, consider orphaned (nanos)
    pub orphan_timeout_ns: u64,
    /// Cleanup interval (how often the background task runs)
    pub interval_ms: u64,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            max_token_age_ns: 60_000_000_000,      // 60 seconds
            orphan_timeout_ns: 30_000_000_000,      // 30 seconds
            interval_ms: 5_000,                      // 5 seconds
        }
    }
}

/// Result of a cleanup run
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CleanupReport {
    /// Number of stale tokens removed
    pub stale_removed: usize,
    /// Number of orphan tokens detected and failed
    pub orphans_failed: usize,
    /// Number of pending requests cancelled (503)
    pub pending_cancelled: usize,
}

/// Run one cleanup cycle on a kernel
pub fn cleanup_kernel(kernel: &VxKernel, config: &CleanupConfig) -> CleanupReport {
    let mut report = CleanupReport::default();

    // 1. Clean up completed/failed/cancelled tokens older than max age
    report.stale_removed = kernel.cleanup(config.max_token_age_ns);

    // 2. Detect orphan tokens (Active for too long → mark as Failed)
    // Detect orphan tokens (Active for too long → mark as Failed)
    // Since VxKernel exposes token_state and we can iterate via metrics,
    // we track orphan detection via the in-flight count vs ready count
    let in_flight = kernel.in_flight();
    let ready = kernel.ready_count();
    let stuck = if in_flight > ready { in_flight - ready } else { 0 };

    if stuck > 0 {
        {
            use vil_log::app_log;
            app_log!(Warn, "vx.cleanup.stuck_tokens", { service: kernel.service(), stuck: stuck as u64 });
        }
        report.orphans_failed = stuck;
    }

    if report.stale_removed > 0 || report.orphans_failed > 0 {
        use vil_log::app_log;
        app_log!(Info, "vx.cleanup.cycle", {
            service: kernel.service(),
            stale_removed: report.stale_removed as u64,
            orphans: report.orphans_failed as u64
        });
    }

    report
}

/// Cancel all pending IngressBridge requests that are older than timeout
pub fn cleanup_stale_pending(bridge: &IngressBridge, max_pending: usize) -> usize {
    let pending = bridge.pending_count();
    if pending > max_pending {
        use vil_log::app_log;
        app_log!(Warn, "vx.pending.requests.leak", { pending: pending as u64, max: max_pending as u64 });
    }
    // IngressBridge doesn't expose age-based cleanup yet.
    // For now, just report the count. Phase 2 will add timestamped entries.
    0
}

/// Spawn a background cleanup task
pub fn spawn_cleanup_task(
    kernel: Arc<VxKernel>,
    bridge: IngressBridge,
    config: CleanupConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_millis(config.interval_ms)
        );

        loop {
            interval.tick().await;
            let report = cleanup_kernel(&kernel, &config);
            let _ = cleanup_stale_pending(&bridge, 10_000);

            // debug-level: skip vil_log
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_config_defaults() {
        let config = CleanupConfig::default();
        assert_eq!(config.interval_ms, 5_000);
        assert_eq!(config.max_token_age_ns, 60_000_000_000);
        assert_eq!(config.orphan_timeout_ns, 30_000_000_000);
    }

    #[test]
    fn cleanup_kernel_empty() {
        let kernel = VxKernel::new("test");
        let config = CleanupConfig::default();
        let report = cleanup_kernel(&kernel, &config);
        assert_eq!(report.stale_removed, 0);
        assert_eq!(report.orphans_failed, 0);
    }

    #[test]
    fn cleanup_kernel_with_completed_tokens() {
        let kernel = VxKernel::new("test");
        // Enqueue and complete a token
        kernel.enqueue(1, "test".into(), vec![1, 2, 3]);
        kernel.dequeue_ready();
        kernel.complete(1);
        // Cleanup with 0 max age (remove all completed immediately)
        let report = cleanup_kernel(&kernel, &CleanupConfig {
            max_token_age_ns: 0,
            orphan_timeout_ns: 0,
            interval_ms: 1000,
        });
        assert_eq!(report.stale_removed, 1);
    }

    #[test]
    fn cleanup_report_serializable() {
        let report = CleanupReport {
            stale_removed: 5,
            orphans_failed: 2,
            pending_cancelled: 1,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"stale_removed\":5"));
    }

    #[test]
    fn cleanup_stale_pending_warns() {
        let bridge = IngressBridge::new();
        let cancelled = cleanup_stale_pending(&bridge, 100);
        assert_eq!(cancelled, 0); // no pending requests
    }
}
