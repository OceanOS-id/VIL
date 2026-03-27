// =============================================================================
// vil_obs::latency — Publish-to-Receive Latency Tracker
// =============================================================================
// Simple bucket-based histogram for tracking latency distribution.
// Bucket boundaries: [0, 1us, 10us, 100us, 1ms, 10ms, 100ms, 1s, inf]
//
// TASK LIST:
// [x] LatencyTracker — bucket-based histogram
// [x] record_ns — record a single latency sample
// [x] LatencySnapshot — percentile and distribution
// [x] Unit tests
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};

/// Bucket boundaries (nanoseconds).
/// [0, 1µs, 10µs, 100µs, 1ms, 10ms, 100ms, 1s, ∞]
const BUCKET_BOUNDS_NS: [u64; NUM_BUCKETS] = [
    0,
    1_000,          // 1µs
    10_000,         // 10µs
    100_000,        // 100µs
    1_000_000,      // 1ms
    10_000_000,     // 10ms
    100_000_000,    // 100ms
    1_000_000_000,  // 1s
    10_000_000_000, // 10s
];

const NUM_BUCKETS: usize = 9;

/// Bucket-based latency histogram. Lock-free via atomics.
#[repr(C)]
#[derive(Debug)]
pub struct LatencyTracker {
    /// Counts per bucket.
    buckets: [AtomicU64; NUM_BUCKETS],
    /// Running sum of latencies (for mean calculation).
    sum_ns: AtomicU64,
    /// Total samples recorded.
    count: AtomicU64,
    /// Minimum latency seen.
    min_ns: AtomicU64,
    /// Maximum latency seen.
    max_ns: AtomicU64,
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyTracker {
    #[doc(alias = "vil_keep")]
    pub fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            sum_ns: AtomicU64::new(0),
            count: AtomicU64::new(0),
            min_ns: AtomicU64::new(u64::MAX),
            max_ns: AtomicU64::new(0),
        }
    }

    /// Record a single latency sample (nanoseconds).
    pub fn record_ns(&self, latency_ns: u64) {
        // Find bucket
        let bucket_idx = Self::bucket_for(latency_ns);
        self.buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);

        // Update aggregates
        self.sum_ns.fetch_add(latency_ns, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);

        // Update min (CAS loop)
        loop {
            let current_min = self.min_ns.load(Ordering::Relaxed);
            if latency_ns >= current_min {
                break;
            }
            if self
                .min_ns
                .compare_exchange_weak(
                    current_min,
                    latency_ns,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break;
            }
        }

        // Update max (CAS loop)
        loop {
            let current_max = self.max_ns.load(Ordering::Relaxed);
            if latency_ns <= current_max {
                break;
            }
            if self
                .max_ns
                .compare_exchange_weak(
                    current_max,
                    latency_ns,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                break;
            }
        }
    }

    /// Find the bucket index for a given latency.
    fn bucket_for(latency_ns: u64) -> usize {
        for (i, &bound) in BUCKET_BOUNDS_NS.iter().enumerate().rev() {
            if latency_ns >= bound {
                return i;
            }
        }
        0
    }

    /// Snapshot of latency distribution.
    pub fn snapshot(&self) -> LatencySnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let sum_ns = self.sum_ns.load(Ordering::Relaxed);
        let min_ns = if count > 0 {
            self.min_ns.load(Ordering::Relaxed)
        } else {
            0
        };
        let max_ns = self.max_ns.load(Ordering::Relaxed);
        let mean_ns = if count > 0 { sum_ns / count } else { 0 };

        let mut bucket_counts = [0u64; NUM_BUCKETS];
        for (i, b) in self.buckets.iter().enumerate() {
            bucket_counts[i] = b.load(Ordering::Relaxed);
        }

        LatencySnapshot {
            count,
            sum_ns,
            min_ns,
            max_ns,
            mean_ns,
            bucket_counts,
        }
    }

    /// Reset tracker.
    pub fn reset(&self) {
        for b in &self.buckets {
            b.store(0, Ordering::Relaxed);
        }
        self.sum_ns.store(0, Ordering::Relaxed);
        self.count.store(0, Ordering::Relaxed);
        self.min_ns.store(u64::MAX, Ordering::Relaxed);
        self.max_ns.store(0, Ordering::Relaxed);
    }
}

/// Immutable snapshot of latency distribution.
#[derive(Clone, Debug, serde::Serialize)]
pub struct LatencySnapshot {
    pub count: u64,
    pub sum_ns: u64,
    pub min_ns: u64,
    pub max_ns: u64,
    pub mean_ns: u64,
    /// Counts per bucket: [0, <1µs, <10µs, <100µs, <1ms, <10ms, <100ms, <1s, ≥1s]
    pub bucket_counts: [u64; NUM_BUCKETS],
}

impl LatencySnapshot {
    /// Bucket labels for display.
    pub fn bucket_labels() -> [&'static str; NUM_BUCKETS] {
        [
            "<1µs",
            "1-10µs",
            "10-100µs",
            "100µs-1ms",
            "1-10ms",
            "10-100ms",
            "100ms-1s",
            "1-10s",
            "≥10s",
        ]
    }
}

impl std::fmt::Display for LatencySnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.count == 0 {
            return write!(f, "latency: no samples");
        }
        write!(
            f,
            "latency: n={} min={}ns mean={}ns max={}ns",
            self.count, self.min_ns, self.mean_ns, self.max_ns
        )?;
        let labels = Self::bucket_labels();
        write!(f, " [")?;
        for (i, &count) in self.bucket_counts.iter().enumerate() {
            if count > 0 {
                write!(f, " {}:{}", labels[i], count)?;
            }
        }
        write!(f, " ]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_assignment() {
        assert_eq!(LatencyTracker::bucket_for(0), 0); // 0ns → bucket 0
        assert_eq!(LatencyTracker::bucket_for(500), 0); // 500ns → bucket 0 (<1µs)
        assert_eq!(LatencyTracker::bucket_for(1_000), 1); // 1µs → bucket 1
        assert_eq!(LatencyTracker::bucket_for(5_000), 1); // 5µs → bucket 1
        assert_eq!(LatencyTracker::bucket_for(10_000), 2); // 10µs → bucket 2
        assert_eq!(LatencyTracker::bucket_for(1_000_000), 4); // 1ms → bucket 4
        assert_eq!(LatencyTracker::bucket_for(2_000_000_000), 7); // 2s → bucket 7 (overflow)
    }

    #[test]
    fn test_record_and_snapshot() {
        let tracker = LatencyTracker::new();
        tracker.record_ns(500); // <1µs
        tracker.record_ns(5_000); // <10µs
        tracker.record_ns(50_000); // <100µs

        let snap = tracker.snapshot();
        assert_eq!(snap.count, 3);
        assert_eq!(snap.min_ns, 500);
        assert_eq!(snap.max_ns, 50_000);
        assert_eq!(snap.mean_ns, (500 + 5_000 + 50_000) / 3);
    }

    #[test]
    fn test_reset() {
        let tracker = LatencyTracker::new();
        tracker.record_ns(1000);
        tracker.reset();
        let snap = tracker.snapshot();
        assert_eq!(snap.count, 0);
        assert_eq!(snap.min_ns, 0);
    }

    #[test]
    fn test_display_empty() {
        let tracker = LatencyTracker::new();
        let s = format!("{}", tracker.snapshot());
        assert!(s.contains("no samples"));
    }

    #[test]
    fn test_display_with_data() {
        let tracker = LatencyTracker::new();
        tracker.record_ns(500);
        tracker.record_ns(5_000);
        let s = format!("{}", tracker.snapshot());
        assert!(s.contains("n=2"));
        assert!(s.contains("min=500ns"));
    }
}
