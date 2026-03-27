// =============================================================================
// vil_registry::registry — Central Ownership Registry
// =============================================================================
// Registry tracks:
//   - ProcessRecord: name, cleanup policy, epoch, alive status
//   - PortRecord: owning process, port name
//   - SampleRecord: owner process, origin port, published state, inflight reads
//
// Key operations:
//   - register_process / register_port / register_sample
//   - mark_published / mark_received / mark_release_read
//   - reclaim_sample / reclaim_orphans_for_process
//   - sample_report / process_report
//
// Crash recovery: reclaim_orphans_for_process collects all samples
// owned by a crashed process and returns their SampleIds so the shared
// store can clean up the data.
//
// TASK LIST:
// [x] ProcessRecord — registered process record
// [x] PortRecord — registered port record
// [x] SampleRecord — sample ownership record
// [x] Registry — CRUD operations
// [x] reclaim_orphans_for_process — crash cleanup
// [x] Reporting — sample_report, process_report
// [x] Unit tests
// =============================================================================

use dashmap::DashMap;
use std::sync::Arc;

use vil_types::{CleanupPolicy, Epoch, PortId, ProcessId, RegionId, SampleId};
use crate::shm_registry::{ProcessSnapshot, PortSnapshot, SampleSnapshot};

/// Record of a registered process in the registry.
#[derive(Clone, Debug)]
pub struct ProcessRecord {
    /// Process name.
    pub name: String,
    /// Cleanup policy on crash.
    pub cleanup: CleanupPolicy,
    /// Current epoch (process generation).
    pub epoch: Epoch,
    /// Whether the process is still alive.
    pub alive: bool,
}

/// Record of a registered port in the registry.
#[derive(Clone, Debug)]
pub struct PortRecord {
    /// Process that owns this port.
    pub process_id: ProcessId,
    /// Port direction.
    pub direction: vil_types::PortDirection,
    /// Port name.
    pub name: String,
}

/// Record of a sample on the shared exchange heap.
#[derive(Clone, Debug)]
pub struct SampleRecord {
    /// Process that owns this sample.
    pub owner: ProcessId,
    /// Origin host of the sample.
    pub origin_host: vil_types::HostId,
    /// Origin port of the sample.
    pub origin_port: PortId,
    /// Whether the sample has been published to a queue.
    pub published: bool,
    /// Number of consumers expected to read this sample.
    pub expected_reads: u32,
    /// Number of consumers that have completed reading.
    pub completed_reads: u32,
    pub offset: u64,
    pub region_id: RegionId,
    pub size: u32,
    pub align: u32,
}

/// Central ownership registry for the entire VIL runtime.
///
/// Concurrent registry via DashMap.
/// Eliminates global Mutex contention on the hot path (publish/recv).
#[derive(Clone, Default)]
pub struct Registry {
    processes: Arc<DashMap<ProcessId, ProcessRecord>>,
    ports: Arc<DashMap<PortId, PortRecord>>,
    samples: Arc<DashMap<SampleId, SampleRecord>>,
    routes: Arc<DashMap<PortId, Vec<PortId>>>,
}

impl Registry {
    /// Create a new empty registry.
    #[doc(alias = "vil_keep")]
    pub fn new() -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
            ports: Arc::new(DashMap::new()),
            samples: Arc::new(DashMap::new()),
            routes: Arc::new(DashMap::new()),
        }
    }

    // --- Process operations ---

    /// Register a new process in the registry.
    #[doc(alias = "vil_keep")]
    pub fn register_process(
        &self,
        process_id: ProcessId,
        name: &str,
        cleanup: CleanupPolicy,
    ) {
        self.processes.insert(
            process_id,
            ProcessRecord {
                name: name.to_string(),
                cleanup,
                epoch: Epoch(1),
                alive: true,
            },
        );
    }

    /// Mark a process as dead (crashed/shutdown).
    pub fn mark_process_dead(&self, process_id: ProcessId) {
        if let Some(mut proc) = self.processes.get_mut(&process_id) {
            proc.alive = false;
        }
    }

    /// Advance process epoch (e.g. on restart).
    pub fn advance_epoch(&self, process_id: ProcessId) {
        if let Some(mut proc) = self.processes.get_mut(&process_id) {
            proc.epoch = Epoch(proc.epoch.0 + 1);
        }
    }

    // --- Port operations ---

    /// Register a port in the registry.
    #[doc(alias = "vil_keep")]
    pub fn register_port(&self, port_id: PortId, process_id: ProcessId, direction: vil_types::PortDirection, name: &str) {
        self.ports.insert(
            port_id,
            PortRecord {
                process_id,
                direction,
                name: name.to_string(),
            },
        );
    }

    // --- Sample operations ---

    /// Register a new sample with owner and origin port.
    pub fn register_sample(
        &self,
        sample_id: SampleId,
        owner: ProcessId,
        origin_host: vil_types::HostId,
        origin_port: PortId,
        expected_reads: u32,
        region_id: RegionId,
        offset: u64,
        size: u32,
        align: u32,
    ) {
        self.samples.insert(
            sample_id,
            SampleRecord {
                owner,
                origin_host,
                origin_port,
                published: false,
                expected_reads,
                completed_reads: 0,
                offset,
                region_id,
                size,
                align,
            },
        );
    }

    /// Mark a sample as published to a queue.
    pub fn mark_published(&self, sample_id: SampleId) {
        if let Some(mut sample) = self.samples.get_mut(&sample_id) {
            sample.published = true;
        }
    }

    /// Mark a sample as being read by a consumer (typically unused in this model,
    /// but API retained for compatibility if needed).
    pub fn mark_received(&self, _sample_id: SampleId) {}

    /// Mark one read as complete (increment completed counter).
    /// Returns true if the sample has been fully read by all targets.
    pub fn mark_release_read(&self, sample_id: SampleId) -> bool {
        if let Some(mut sample) = self.samples.get_mut(&sample_id) {
            sample.completed_reads = sample.completed_reads.saturating_add(1);
            sample.completed_reads >= sample.expected_reads
        } else {
            false
        }
    }

    /// Remove a sample from the registry (reclaim).
    pub fn reclaim_sample(&self, sample_id: SampleId) {
        self.samples.remove(&sample_id);
    }

    /// Reclaim all orphan samples owned by a dead process.
    ///
    /// Returns a list of SampleIds that need to be cleaned from the shared store.
    /// This is a core part of VIL crash recovery.
    pub fn reclaim_orphans_for_process(&self, process_id: ProcessId) -> Vec<SampleId> {
        let mut orphans = Vec::new();
        self.samples.retain(|id, record| {
            if record.owner == process_id {
                orphans.push(*id);
                false // remove from map
            } else {
                true // keep in map
            }
        });
        orphans
    }

    // --- Reporting ---

    /// Report all samples with ownership status (Snapshot).
    pub fn sample_report(&self) -> Vec<SampleSnapshot> {
        self.samples
            .iter()
            .map(|r| {
                let s = r.value();
                SampleSnapshot {
                    id: *r.key(),
                    owner: s.owner,
                    origin_host: s.origin_host,
                    active: true, // Samples in this map are always active
                    published: s.published,
                    reads: (s.completed_reads, s.expected_reads),
                    offset: s.offset,
                    region_id: s.region_id,
                    size: s.size,
                    align: s.align,
                }
            })
            .collect()
    }

    /// Report all registered processes (Snapshot).
    pub fn process_report(&self) -> Vec<ProcessSnapshot> {
        self.processes
            .iter()
            .map(|r| {
                let p = r.value();
                ProcessSnapshot {
                    id: *r.key(),
                    name: p.name.clone(),
                    alive: p.alive,
                    epoch: p.epoch,
                }
            })
            .collect()
    }

    /// Report all registered ports (Snapshot).
    pub fn port_report(&self) -> Vec<PortSnapshot> {
        self.ports
            .iter()
            .map(|r| {
                let p = r.value();
                PortSnapshot {
                    id: *r.key(),
                    process_id: p.process_id,
                    direction: p.direction,
                    name: p.name.clone(),
                }
            })
            .collect()
    }


    /// Number of registered samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    // --- Routing operations ---

    #[doc(alias = "vil_keep")]
    pub fn register_route(&self, from: PortId, to: PortId) {
        self.routes.entry(from).or_default().push(to);
    }

    pub fn unregister_route(&self, from: PortId, to: PortId) {
        if let Some(mut targets) = self.routes.get_mut(&from) {
            targets.retain(|t| *t != to);
        }
    }

    pub fn clear_routes(&self, from: PortId) {
        self.routes.remove(&from);
    }

    pub fn get_routes(&self, from: PortId) -> Vec<PortId> {
        self.routes.get(&from).map(|r| r.value().clone()).unwrap_or_default()
    }
}

impl vil_shm::DefragRegistry for Registry {
    fn get_active_samples(&self, region_id: vil_types::RegionId) -> Vec<vil_shm::DefragSample> {
        self.sample_report()
            .into_iter()
            .filter(|s| s.region_id == region_id)
            .map(|s| vil_shm::DefragSample {
                id: s.id,
                offset: s.offset,
                size: s.size as usize,
                align: s.align as usize,
            })
            .collect()
    }

    fn update_offset(&self, sample_id: vil_types::SampleId, new_offset: u64) {
        if let Some(mut sample) = self.samples.get_mut(&sample_id) {
            sample.offset = new_offset;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_report_process() {
        let reg = Registry::new();
        reg.register_process(ProcessId(1), "producer", CleanupPolicy::ReclaimOrphans);

        let procs = reg.process_report();
        assert_eq!(procs.len(), 1);
        assert_eq!(procs[0].name, "producer");
        assert!(procs[0].alive);
        assert_eq!(procs[0].id, ProcessId(1));
    }

    #[test]
    fn test_mark_process_dead() {
        let reg = Registry::new();
        reg.register_process(ProcessId(1), "worker", CleanupPolicy::ReclaimOrphans);
        reg.mark_process_dead(ProcessId(1));

        let procs = reg.process_report();
        assert!(!procs[0].alive);
    }

    #[test]
    fn test_sample_lifecycle() {
        let reg = Registry::new();
        reg.register_sample(SampleId(1), ProcessId(1), vil_types::HostId(0), PortId(1), 2, RegionId(0), 123, 1024, 8); // 2 consumers

        // Not yet published
        let report = reg.sample_report();
        assert_eq!(report.len(), 1);
        assert!(!report[0].published);
        assert_eq!(report[0].reads.0, 0);

        // Publish
        reg.mark_published(SampleId(1));
        let report = reg.sample_report();
        assert!(report[0].published);

        // Received by 1st consumer
        let reclaimed_1 = reg.mark_release_read(SampleId(1));
        assert!(!reclaimed_1);
        let report = reg.sample_report();
        assert_eq!(report[0].reads.0, 1);

        // Received by 2nd consumer
        let reclaimed_2 = reg.mark_release_read(SampleId(1));
        assert!(reclaimed_2);
        let report = reg.sample_report();
        assert_eq!(report[0].reads.0, 2);
    }

    #[test]
    fn test_reclaim_orphans() {
        let reg = Registry::new();
        let pid = ProcessId(42);

        reg.register_sample(SampleId(1), pid, vil_types::HostId(0), PortId(1), 1, RegionId(0), 1, 1024, 8);
        reg.register_sample(SampleId(2), pid, vil_types::HostId(0), PortId(1), 1, RegionId(0), 2, 1024, 8);
        reg.register_sample(SampleId(3), ProcessId(99), vil_types::HostId(0), PortId(2), 1, RegionId(0), 3, 1024, 8); // different process

        assert_eq!(reg.sample_count(), 3);

        let orphans = reg.reclaim_orphans_for_process(pid);
        assert_eq!(orphans.len(), 2);
        assert_eq!(reg.sample_count(), 1); // only pid 99's sample remains
    }

    #[test]
    fn test_advance_epoch() {
        let reg = Registry::new();
        reg.register_process(ProcessId(1), "restartable", CleanupPolicy::ReclaimOrphans);
        reg.advance_epoch(ProcessId(1));

        let procs = reg.process_report();
        assert_eq!(procs[0].id, ProcessId(1));
    }

    #[test]
    fn test_reclaim_sample_directly() {
        let reg = Registry::new();
        reg.register_sample(SampleId(1), ProcessId(1), vil_types::HostId(0), PortId(1), 1, RegionId(0), 0, 1024, 8);
        assert_eq!(reg.sample_count(), 1);

        reg.reclaim_sample(SampleId(1));
        assert_eq!(reg.sample_count(), 0);
    }
}
