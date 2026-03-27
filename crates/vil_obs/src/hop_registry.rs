// =============================================================================
// vil_obs::hop_registry — Named Hop Latency Registry
// =============================================================================
// Tracks per-label latency histograms for processes annotated with
// `#[latency_marker("label")]` and `#[trace_hop]`.
//
// Design: RwLock<HashMap<String, LatencyTracker>> — write only on new label insertion.
// Multiple concurrent readers can snapshot without contention.
// =============================================================================

use crate::latency::{LatencySnapshot, LatencyTracker};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry of per-label latency trackers.
/// Each label corresponds to a `#[latency_marker("label")]`-annotated process.
#[derive(Default, Clone)]
pub struct HopLatencyRegistry {
    inner: Arc<RwLock<HashMap<String, Arc<LatencyTracker>>>>,
}

impl HopLatencyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a hop latency sample (ns) for the given label.
    /// If the label doesn't exist yet, a new tracker is created.
    pub fn record(&self, label: &str, latency_ns: u64) {
        // Fast path: label already exists
        {
            let r = self.inner.read().unwrap();
            if let Some(tracker) = r.get(label) {
                tracker.record_ns(latency_ns);
                return;
            }
        }
        // Slow path: insert new tracker
        let mut w = self.inner.write().unwrap();
        let tracker = w
            .entry(label.to_string())
            .or_insert_with(|| Arc::new(LatencyTracker::new()))
            .clone();
        tracker.record_ns(latency_ns);
    }

    /// Record an anonymous hop (no label) — used by `#[trace_hop]` without `#[latency_marker]`.
    pub fn record_anonymous_hop(&self, latency_ns: u64) {
        self.record("__anonymous__", latency_ns);
    }

    /// Snapshot all labels.
    pub fn snapshot_all(&self) -> HashMap<String, LatencySnapshot> {
        let r = self.inner.read().unwrap();
        r.iter().map(|(k, v)| (k.clone(), v.snapshot())).collect()
    }

    /// Snapshot a single label. Returns `None` if not recorded yet.
    pub fn snapshot(&self, label: &str) -> Option<LatencySnapshot> {
        let r = self.inner.read().unwrap();
        r.get(label).map(|t| t.snapshot())
    }

    /// All registered labels.
    pub fn labels(&self) -> Vec<String> {
        self.inner.read().unwrap().keys().cloned().collect()
    }

    /// Reset all trackers.
    pub fn reset_all(&self) {
        let r = self.inner.read().unwrap();
        for t in r.values() {
            t.reset();
        }
    }
}

impl std::fmt::Debug for HopLatencyRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labels = self.labels();
        write!(f, "HopLatencyRegistry {{ labels: {:?} }}", labels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_snapshot() {
        let reg = HopLatencyRegistry::new();
        reg.record("inference", 1_500); // 1.5µs
        reg.record("inference", 2_000); // 2µs
        reg.record("auth", 500);

        let snap = reg.snapshot("inference").unwrap();
        assert_eq!(snap.count, 2);
        assert_eq!(snap.min_ns, 1_500);
        assert_eq!(snap.max_ns, 2_000);

        let auth_snap = reg.snapshot("auth").unwrap();
        assert_eq!(auth_snap.count, 1);
    }

    #[test]
    fn test_anonymous_hop() {
        let reg = HopLatencyRegistry::new();
        reg.record_anonymous_hop(3_000);
        let snap = reg.snapshot("__anonymous__").unwrap();
        assert_eq!(snap.count, 1);
    }

    #[test]
    fn test_snapshot_all() {
        let reg = HopLatencyRegistry::new();
        reg.record("a", 1000);
        reg.record("b", 2000);
        let all = reg.snapshot_all();
        assert!(all.contains_key("a"));
        assert!(all.contains_key("b"));
    }

    #[test]
    fn test_reset_all() {
        let reg = HopLatencyRegistry::new();
        reg.record("x", 5000);
        reg.reset_all();
        let snap = reg.snapshot("x").unwrap();
        assert_eq!(snap.count, 0);
    }
}
