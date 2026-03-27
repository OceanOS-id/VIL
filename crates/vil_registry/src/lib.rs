// =============================================================================
// vil_registry — Ownership Registry
// =============================================================================
// Central registry tracking and enforcing ownership lifecycle:
//   - Process: liveness, epoch, cleanup policy
//   - Port: owning process
//   - Sample: owner, published state, inflight read count
//
// Key features:
//   - Crash recovery: reclaim orphan samples when a process dies
//   - Epoch tracking: detect process restarts
//   - Audit trail: report all samples and ownership status
//
// Modules:
//   registry.rs — Central registry (process, port, sample tracking)
//   epoch.rs    — EpochTracker (per-process crash detection)
//
// TASK LIST:
// [x] ProcessRecord, PortRecord, SampleRecord
// [x] Registry — register/mark/reclaim operations
// [x] Orphan reclaim for crashed process
// [x] EpochTracker — advance/current/is_alive
// [x] Unit tests
// [ ] TODO(future): recursive ownership tracking
// [ ] TODO(future): bottom marking for subtree transfer
// [ ] TODO(future): lock-free registry for hot path
// =============================================================================

pub mod registry;
pub mod epoch;
pub mod shm_registry;

pub use registry::*;
pub use epoch::EpochTracker;
pub use shm_registry::{
    SharedRegistryLayout, ShmRegistry,
    ProcessSnapshot, PortSnapshot, SampleSnapshot
};
pub use vil_types::{
    Descriptor, Loaned, LoanedRead, PortId, ProcessId, ProcessSpec, Published,
    QueueKind, RegionId, SampleId, VSlice, GenericToken, MessageContract,
};
