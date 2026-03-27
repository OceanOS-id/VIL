// =============================================================================
// vil_obs::observer — Runtime Observer (Event Callback Sink)
// =============================================================================
// RuntimeObserver is a callback sink that receives TraceEvents and
// distributes them to subscribers. This is the hook point for
// all observability tooling.
//
// Pattern: Observer holds a list of callback closures.
// Each time an event occurs, all callbacks are invoked synchronously.
//
// TASK LIST:
// [x] RuntimeObserver — callback-based event sink
// [x] subscribe — tambah callback
// [x] emit — emit event to all subscribers
// [x] emit_and_count — emit + increment RuntimeCounters
// [x] ObservabilityHub — combined observer + counters + latency
// [x] Unit tests
// =============================================================================

use std::sync::Mutex;

use crate::counters::RuntimeCounters;
use crate::events::TraceEvent;
use crate::latency::LatencyTracker;

/// Callback type invoked when an event occurs.
pub type EventCallback = Box<dyn Fn(&TraceEvent) + Send + Sync>;

use std::sync::atomic::{AtomicBool, Ordering};

/// Event callback sink. Thread-safe via Arc<Mutex>.
pub struct RuntimeObserver {
    callbacks: Mutex<Vec<EventCallback>>,
    has_callbacks: AtomicBool,
}

impl RuntimeObserver {
    #[doc(alias = "vil_keep")]
    pub fn new() -> Self {
        Self {
            callbacks: Mutex::new(Vec::new()),
            has_callbacks: AtomicBool::new(false),
        }
    }

    pub fn has_callbacks(&self) -> bool {
        self.has_callbacks.load(Ordering::Relaxed)
    }

    /// Add a callback subscriber.
    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&TraceEvent) + Send + Sync + 'static,
    {
        let mut cbs = self.callbacks.lock().expect("observer lock poisoned");
        cbs.push(Box::new(callback));
        self.has_callbacks.store(true, Ordering::SeqCst);
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, event: &TraceEvent) {
        // FAST PATH: skip lock if no one is listening.
        if !self.has_callbacks() {
            return;
        }

        let cbs = self.callbacks.lock().expect("observer lock poisoned");
        for cb in cbs.iter() {
            cb(event);
        }
    }

    /// Number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        let cbs = self.callbacks.lock().expect("observer lock poisoned");
        cbs.len()
    }
}

impl Default for RuntimeObserver {
    fn default() -> Self {
        Self::new()
    }
}

/// ObservabilityHub — all-in-one observability facade.
///
/// Combines:
/// - RuntimeObserver (event callbacks)
/// - RuntimeCounters (atomic counters)
/// - LatencyTracker (latency histogram)
///
/// One instance per RuntimeWorld.
pub struct ObservabilityHub {
    pub observer: RuntimeObserver,
    pub counters: RuntimeCounters,
    pub latency: LatencyTracker,
}

impl ObservabilityHub {
    #[doc(alias = "vil_keep")]
    pub fn new() -> Self {
        Self {
            observer: RuntimeObserver::new(),
            counters: RuntimeCounters::new(),
            latency: LatencyTracker::new(),
        }
    }

    pub fn has_callbacks(&self) -> bool {
        self.observer.has_callbacks()
    }

    /// Emit an event and auto-update counters based on event type.
    pub fn emit(&self, event: &TraceEvent) {
        // Update counters
        match event {
            TraceEvent::Published { .. } => self.counters.inc_publishes(),
            TraceEvent::Received { latency_ns, .. } => {
                self.counters.inc_receives();
                if let Some(lat) = latency_ns {
                    self.latency.record_ns(*lat);
                }
            }
            TraceEvent::Dropped { .. } => self.counters.inc_drops(),
            TraceEvent::ProcessCrashed {
                orphan_count,
                drained_count,
                ..
            } => {
                self.counters.inc_crashes();
                self.counters.add_orphans_reclaimed(*orphan_count as u64);
                self.counters.add_descriptors_drained(*drained_count as u64);
            }
            _ => {}
        }

        // Forward to observer callbacks
        self.observer.emit(event);
    }

    /// Record manual latency measurement.
    pub fn record_latency(&self, latency_ns: u64) {
        self.latency.record_ns(latency_ns);
    }

    /// Get a snapshot of the current latency distribution.
    pub fn latency_snapshot(&self) -> crate::latency::LatencySnapshot {
        self.latency.snapshot()
    }
}

impl Default for ObservabilityHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::now_ns;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Instant;
    use vil_types::{PortId, ProcessId, SampleId};

    #[test]
    fn test_observer_subscribe_and_emit() {
        let obs = RuntimeObserver::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c = count.clone();
        obs.subscribe(move |_event| {
            c.fetch_add(1, Ordering::Relaxed);
        });

        assert_eq!(obs.subscriber_count(), 1);

        let event = TraceEvent::Published {
            ts_ns: now_ns(),
            sample_id: SampleId(1),
            origin_port: PortId(1),
            owner: ProcessId(1),
            instant: Instant::now(),
        };

        obs.emit(&event);
        obs.emit(&event);
        assert_eq!(count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_hub_auto_counters() {
        let hub = ObservabilityHub::new();

        // Emit a publish event
        hub.emit(&TraceEvent::Published {
            ts_ns: now_ns(),
            sample_id: SampleId(1),
            origin_port: PortId(1),
            owner: ProcessId(1),
            instant: Instant::now(),
        });

        // Emit a receive event with latency
        hub.emit(&TraceEvent::Received {
            ts_ns: now_ns(),
            sample_id: SampleId(1),
            target_port: PortId(2),
            latency_ns: Some(5_000), // 5µs
        });

        let snap = hub.counters.snapshot();
        assert_eq!(snap.publishes, 1);
        assert_eq!(snap.receives, 1);

        let lat = hub.latency.snapshot();
        assert_eq!(lat.count, 1);
        assert_eq!(lat.min_ns, 5_000);
    }

    #[test]
    fn test_hub_crash_counters() {
        let hub = ObservabilityHub::new();

        hub.emit(&TraceEvent::ProcessCrashed {
            ts_ns: now_ns(),
            process_id: ProcessId(1),
            orphan_count: 5,
            drained_count: 3,
        });

        let snap = hub.counters.snapshot();
        assert_eq!(snap.crashes, 1);
        assert_eq!(snap.orphans_reclaimed, 5);
        assert_eq!(snap.descriptors_drained, 3);
    }

    #[test]
    fn test_multiple_subscribers() {
        let obs = RuntimeObserver::new();
        let c1 = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::new(AtomicUsize::new(0));

        let cc1 = c1.clone();
        obs.subscribe(move |_| {
            cc1.fetch_add(1, Ordering::Relaxed);
        });

        let cc2 = c2.clone();
        obs.subscribe(move |_| {
            cc2.fetch_add(10, Ordering::Relaxed);
        });

        obs.emit(&TraceEvent::ProcessShutdown {
            ts_ns: now_ns(),
            process_id: ProcessId(1),
        });

        assert_eq!(c1.load(Ordering::Relaxed), 1);
        assert_eq!(c2.load(Ordering::Relaxed), 10);
    }
}
