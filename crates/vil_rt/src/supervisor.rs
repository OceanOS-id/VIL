// =============================================================================
// vil_rt::supervisor — Process Supervisor & Crash Cleanup
// =============================================================================
// Supervisor coordinates VIL crash recovery:
//
//   1. DETECT: Process died (epoch changed / explicit shutdown)
//   2. RECLAIM: Collect orphan samples from registry
//   3. CLEANUP STORE: Remove orphan data from shared store
//   4. DRAIN QUEUES: Mark descriptors from crashed process as invalid
//   5. ADVANCE EPOCH: Increment epoch so consumers know data is stale
//
// This is VIL's "garbage collector" — but deterministic, not
// generational. Cleanup occurs when a process dies, not periodically.
//
// TASK LIST:
// [x] Supervisor struct
// [x] shutdown_process — graceful shutdown + cleanup
// [x] crash_process — simulate crash + full cleanup
// [x] cleanup_for_process — internal cleanup orchestration
// [x] drain_invalid_descriptors — remove crashed process descriptors
// [x] CleanupReport — laporan hasil cleanup
// [x] Unit tests (full crash cycle)
// [ ] TODO(future): periodic liveness probe
// [ ] TODO(future): recursive ownership cleanup
// [ ] TODO(future): configurable cleanup policy execution
// =============================================================================

use dashmap::DashMap;
use std::sync::Arc;

use vil_log::{system_log, types::SystemPayload};
use vil_queue::QueueBackend;
use vil_registry::Registry;
use vil_shm::SharedStore;
use vil_types::{PortId, ProcessId, SampleId};

/// Report of cleanup results after process shutdown/crash.
#[derive(Clone, Debug, Default)]
pub struct CleanupReport {
    /// Process ID that was cleaned up.
    pub process_id: ProcessId,
    /// Sample IDs reclaimed from the registry.
    pub reclaimed_samples: Vec<SampleId>,
    /// Number of samples removed from the shared store.
    pub store_removals: usize,
    /// Number of descriptors drained from queues.
    pub drained_descriptors: usize,
}

impl CleanupReport {
    /// Whether anything was cleaned up.
    pub fn is_empty(&self) -> bool {
        self.reclaimed_samples.is_empty()
            && self.store_removals == 0
            && self.drained_descriptors == 0
    }
}

impl std::fmt::Display for CleanupReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CleanupReport(pid={}, reclaimed={}, store_removed={}, drained={})",
            self.process_id,
            self.reclaimed_samples.len(),
            self.store_removals,
            self.drained_descriptors
        )
    }
}

/// Process Supervisor — crash detection and cleanup orchestration.
///
/// Works with Registry + SharedStore + Queues to ensure no resources
/// leak when a process dies.
pub struct Supervisor {
    registry: Registry,
    store: SharedStore,
    queues: Arc<DashMap<PortId, Box<dyn QueueBackend>>>,
}

impl Supervisor {
    /// Create a new supervisor.
    pub(crate) fn new(
        registry: Registry,
        store: SharedStore,
        queues: Arc<DashMap<PortId, Box<dyn QueueBackend>>>,
    ) -> Self {
        Self {
            registry,
            store,
            queues,
        }
    }

    /// Graceful shutdown: process has completed normally.
    ///
    /// Clean up all resources owned by the process:
    /// 1. Mark process dead di registry
    /// 2. Reclaim orphan samples
    /// 3. Remove from shared store
    /// 4. Drain invalid descriptors from queues
    pub fn shutdown_process(&self, process_id: ProcessId) -> CleanupReport {
        system_log!(Info, SystemPayload {
            event_type: 5, // shutdown
            exit_code: 0,
            ..SystemPayload::default()
        });
        self.registry.mark_process_dead(process_id);
        self.cleanup_for_process(process_id)
    }

    /// Crash cleanup: process died unexpectedly.
    ///
    /// Same as shutdown but also advances epoch to mark
    /// the previous generation as invalid.
    pub fn crash_process(&self, process_id: ProcessId) -> CleanupReport {
        system_log!(Warn, SystemPayload {
            event_type: 3, // panic / crash
            exit_code: 1,
            ..SystemPayload::default()
        });
        self.registry.mark_process_dead(process_id);
        let report = self.cleanup_for_process(process_id);
        self.registry.advance_epoch(process_id);
        report
    }

    /// Internal cleanup orchestration.
    fn cleanup_for_process(&self, process_id: ProcessId) -> CleanupReport {
        // 1. Reclaim orphan samples from registry
        let reclaimed_samples = self.registry.reclaim_orphans_for_process(process_id);

        // 2. Remove data from shared store
        let mut store_removals = 0;
        for sample_id in &reclaimed_samples {
            if self.store.remove(*sample_id) {
                store_removals += 1;
            }
        }

        // 3. Drain descriptors referencing reclaimed samples
        let drained_descriptors = self.drain_invalid_descriptors(&reclaimed_samples);

        CleanupReport {
            process_id,
            reclaimed_samples,
            store_removals,
            drained_descriptors,
        }
    }

    /// Drain descriptors from all queues that reference invalid sample IDs.
    ///
    /// Strategy: pop all descriptors from queue, filter out invalid ones,
    /// push back valid ones. This is O(n) per queue — acceptable since
    /// crashes are rare events.
    fn drain_invalid_descriptors(&self, invalid_samples: &[SampleId]) -> usize {
        if invalid_samples.is_empty() {
            return 0;
        }

        let mut total_drained = 0;

        for r in self.queues.iter() {
            let queue = r.value();
            // Pop all descriptors
            let mut valid = Vec::new();
            while let Some(desc) = queue.try_pop() {
                if invalid_samples.contains(&desc.sample_id) {
                    total_drained += 1;
                } else {
                    valid.push(desc);
                }
            }
            // Push back valid ones
            for desc in valid {
                queue.push(desc);
            }
        }

        total_drained
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_queue::DescriptorQueue;
    use vil_types::*;

    fn make_desc(sample_id: u64, port: u64) -> Descriptor {
        Descriptor {
            sample_id: SampleId(sample_id),
            origin_host: HostId(0),
            origin_port: PortId(port),
            lineage_id: sample_id * 10,
            publish_ts: 0,
        }
    }

    fn setup() -> (Supervisor, PortId) {
        let registry = Registry::new();
        let store = SharedStore::new();
        let port_id = PortId(1);
        let queue: Box<dyn QueueBackend> = Box::new(DescriptorQueue::new());
        let queues = Arc::new(DashMap::new());
        queues.insert(port_id, queue);

        let supervisor = Supervisor::new(registry.clone(), store.clone(), queues);
        // Also register the process in the registry
        registry.register_process(ProcessId(1), "producer", CleanupPolicy::ReclaimOrphans);

        (supervisor, port_id)
    }

    #[test]
    fn test_shutdown_reclaims_samples() {
        let (supervisor, _port_id) = setup();
        let pid = ProcessId(1);

        // Register samples owned by the process
        supervisor.registry.register_sample(SampleId(1), pid, HostId(0), PortId(1), 1, RegionId(0), 0, 1024, 8);
        supervisor.registry.register_sample(SampleId(2), pid, HostId(0), PortId(1), 1, RegionId(0), 0, 1024, 8);
        supervisor.store.insert_typed(SampleId(1), 42u64);
        supervisor.store.insert_typed(SampleId(2), 99u64);

        let report = supervisor.shutdown_process(pid);
        assert_eq!(report.reclaimed_samples.len(), 2);
        assert_eq!(report.store_removals, 2);
        assert!(!supervisor.store.contains(SampleId(1)));
        assert!(!supervisor.store.contains(SampleId(2)));
    }

    #[test]
    fn test_crash_advances_epoch() {
        let (supervisor, _port_id) = setup();
        let pid = ProcessId(1);

        let procs_before = supervisor.registry.process_report();
        let epoch_before = procs_before.iter().find(|p| p.id == pid).unwrap().epoch;

        supervisor.crash_process(pid);

        let procs_after = supervisor.registry.process_report();
        let epoch_after = procs_after.iter().find(|p| p.id == pid).unwrap().epoch;

        assert!(epoch_after.0 > epoch_before.0, "epoch should advance after crash");
    }

    #[test]
    fn test_drain_invalid_descriptors() {
        let (supervisor, port_id) = setup();
        let pid = ProcessId(1);

        // Register 2 samples from pid, 1 from different pid
        supervisor.registry.register_sample(SampleId(1), pid, HostId(0), PortId(1), 1, RegionId(0), 0, 1024, 8);
        supervisor.registry.register_sample(SampleId(2), pid, HostId(0), PortId(1), 1, RegionId(0), 0, 1024, 8);
        supervisor.registry.register_sample(SampleId(3), ProcessId(99), HostId(0), PortId(1), 1, RegionId(0), 0, 1024, 8);
        supervisor.store.insert_typed(SampleId(1), 1u64);
        supervisor.store.insert_typed(SampleId(2), 2u64);
        supervisor.store.insert_typed(SampleId(3), 3u64);

        // Push descriptors for all 3 into queue
        let q = supervisor.queues.get(&port_id).unwrap();
        q.push(make_desc(1, 1));
        q.push(make_desc(2, 1));
        q.push(make_desc(3, 1)); // from pid 99 — should survive

        // Crash pid 1
        let report = supervisor.crash_process(pid);

        assert_eq!(report.reclaimed_samples.len(), 2);
        assert_eq!(report.store_removals, 2);
        assert_eq!(report.drained_descriptors, 2);

        // Queue should only have descriptor for sample 3
        let q = supervisor.queues.get(&port_id).unwrap();
        assert_eq!(q.len(), 1);
        let remaining = q.try_pop().unwrap();
        assert_eq!(remaining.sample_id, SampleId(3));
    }

    #[test]
    fn test_empty_cleanup_report() {
        let (supervisor, _port_id) = setup();
        let report = supervisor.shutdown_process(ProcessId(999)); // nonexistent
        assert!(report.is_empty());
    }

    #[test]
    fn test_cleanup_display() {
        let report = CleanupReport {
            process_id: ProcessId(1),
            reclaimed_samples: vec![SampleId(1), SampleId(2)],
            store_removals: 2,
            drained_descriptors: 1,
        };
        let s = format!("{}", report);
        assert!(s.contains("pid=Process(1)"));
        assert!(s.contains("reclaimed=2"));
        assert!(s.contains("store_removed=2"));
        assert!(s.contains("drained=1"));
    }
}
