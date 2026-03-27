// =============================================================================
// vil_obs::counters — Atomic Runtime Counters
// =============================================================================
// Live counters for the VIL runtime. Thread-safe via AtomicU64.
// Queryable at any time without locks — O(1) reads.
//
// TASK LIST:
// [x] RuntimeCounters — publish/recv/drop/crash/reclaim counters
// [x] Atomic increment/read
// [x] snapshot() → CounterSnapshot
// [x] reset()
// [x] Unit tests
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};

/// Live runtime counters. All counters are atomic — safe for concurrent access.
#[repr(C)]
#[derive(Debug, Default)]
pub struct RuntimeCounters {
    /// Total samples published.
    pub publishes: AtomicU64,
    /// Total samples received by consumers.
    pub receives: AtomicU64,
    /// Total samples dropped.
    pub drops: AtomicU64,
    /// Total crashes detected.
    pub crashes: AtomicU64,
    /// Total orphan samples reclaimed.
    pub orphans_reclaimed: AtomicU64,
    /// Total descriptors drained from queues during crash recovery.
    pub descriptors_drained: AtomicU64,
    /// Total cross-host pulls (RDMA simulation).
    pub net_pulls: AtomicU64,
    /// Total failover events (re-routing due to failure).
    pub failover_events: AtomicU64,
    /// Total hop latency samples recorded.
    pub hops: AtomicU64,
}

/// Immutable point-in-time snapshot of all counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct CounterSnapshot {
    pub publishes: u64,
    pub receives: u64,
    pub drops: u64,
    pub crashes: u64,
    pub orphans_reclaimed: u64,
    pub descriptors_drained: u64,
    pub net_pulls: u64,
    pub failover_events: u64,
    /// Total hop latency samples recorded.
    pub hops: u64,
}

impl RuntimeCounters {
    #[doc(alias = "vil_keep")]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc_publishes(&self) {
        self.publishes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_receives(&self) {
        self.receives.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_drops(&self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_crashes(&self) {
        self.crashes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_orphans_reclaimed(&self, count: u64) {
        self.orphans_reclaimed.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_descriptors_drained(&self, count: u64) {
        self.descriptors_drained.fetch_add(count, Ordering::Relaxed);
    }

    pub fn inc_net_pulls(&self) {
        self.net_pulls.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_failover_events(&self) {
        self.failover_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a hop traversal.
    pub fn inc_hops(&self) {
        self.hops.fetch_add(1, Ordering::Relaxed);
    }

    /// Snapshot all counters.
    pub fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            publishes: self.publishes.load(Ordering::Relaxed),
            receives: self.receives.load(Ordering::Relaxed),
            drops: self.drops.load(Ordering::Relaxed),
            crashes: self.crashes.load(Ordering::Relaxed),
            orphans_reclaimed: self.orphans_reclaimed.load(Ordering::Relaxed),
            descriptors_drained: self.descriptors_drained.load(Ordering::Relaxed),
            net_pulls: self.net_pulls.load(Ordering::Relaxed),
            failover_events: self.failover_events.load(Ordering::Relaxed),
            hops: self.hops.load(Ordering::Relaxed),
        }
    }

    /// Reset all counters to 0.
    pub fn reset(&self) {
        self.publishes.store(0, Ordering::Relaxed);
        self.receives.store(0, Ordering::Relaxed);
        self.drops.store(0, Ordering::Relaxed);
        self.crashes.store(0, Ordering::Relaxed);
        self.orphans_reclaimed.store(0, Ordering::Relaxed);
        self.descriptors_drained.store(0, Ordering::Relaxed);
        self.net_pulls.store(0, Ordering::Relaxed);
        self.failover_events.store(0, Ordering::Relaxed);
        self.hops.store(0, Ordering::Relaxed);
    }
}

impl std::fmt::Display for CounterSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pub={} recv={} drop={} crash={} orphan={} drain={} pull={} failover={}",
            self.publishes,
            self.receives,
            self.drops,
            self.crashes,
            self.orphans_reclaimed,
            self.descriptors_drained,
            self.net_pulls,
            self.failover_events
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inc_and_snapshot() {
        let c = RuntimeCounters::new();
        c.inc_publishes();
        c.inc_publishes();
        c.inc_receives();
        c.inc_drops();
        c.inc_crashes();
        c.add_orphans_reclaimed(5);
        c.add_descriptors_drained(3);

        let s = c.snapshot();
        assert_eq!(s.publishes, 2);
        assert_eq!(s.receives, 1);
        assert_eq!(s.drops, 1);
        assert_eq!(s.crashes, 1);
        assert_eq!(s.orphans_reclaimed, 5);
        assert_eq!(s.descriptors_drained, 3);
    }

    #[test]
    fn test_reset() {
        let c = RuntimeCounters::new();
        c.inc_publishes();
        c.inc_receives();
        c.reset();
        let s = c.snapshot();
        assert_eq!(s, CounterSnapshot::default());
    }

    #[test]
    fn test_display() {
        let s = CounterSnapshot {
            publishes: 10,
            receives: 8,
            drops: 1,
            crashes: 0,
            orphans_reclaimed: 1,
            descriptors_drained: 2,
            net_pulls: 0,
            failover_events: 0,
            hops: 0,
        };
        let d = format!("{}", s);
        assert!(d.contains("pub=10"));
        assert!(d.contains("recv=8"));
    }

    #[test]
    fn test_concurrent_counters() {
        use std::sync::Arc;
        use std::thread;

        let c = Arc::new(RuntimeCounters::new());
        let mut handles = vec![];

        for _ in 0..8 {
            let cc = c.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    cc.inc_publishes();
                    cc.inc_receives();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let s = c.snapshot();
        assert_eq!(s.publishes, 8000);
        assert_eq!(s.receives, 8000);
    }
}
