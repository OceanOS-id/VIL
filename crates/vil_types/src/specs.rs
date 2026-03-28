// =============================================================================
// vil_types::specs — Composite Specifications
// =============================================================================
// Specification structs combining enums and IDs into complete contracts
// for ports, processes, descriptors, and message metadata.
//
// Used by vil_rt for process and port registration,
// and by vil_ir (future) for IR representation.
//
// TASK LIST:
// [x] ObservabilitySpec — tracing/metrics/lineage configuration per entity
// [x] PortSpec — complete specification for a single port
// [x] ProcessSpec — complete specification for a single process
// [x] Descriptor — small payload traveling in queues (not large data!)
// [x] MessageMeta — static message contract metadata
// =============================================================================

use crate::enums::*;
use crate::ids::*;
use serde::{Deserialize, Serialize};

/// Observability specification attached to a process, port, or message.
///
/// Default: tracing=on, metrics=on, lineage=on, audit=off, latency=Normal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservabilitySpec {
    pub tracing: bool,
    pub metrics: bool,
    pub lineage: bool,
    pub audit_sample_handoff: bool,
    pub latency_class: LatencyClass,
}

impl Default for ObservabilitySpec {
    fn default() -> Self {
        Self {
            tracing: true,
            metrics: true,
            lineage: true,
            audit_sample_handoff: false,
            latency_class: LatencyClass::Normal,
        }
    }
}

/// Complete specification of a single communication port.
///
/// A port is an explicit data exchange point with direction, policy, and queue.
/// This is a contract that the runtime must not violate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortSpec {
    /// Port name (static string, e.g. "send_frame").
    pub name: &'static str,
    /// Port direction: in, out, request, response.
    pub direction: PortDirection,
    /// Queue kind: SPSC (default) or MPMC (opt-in).
    pub queue: QueueKind,
    /// Queue capacity in number of descriptors.
    pub capacity: usize,
    /// Policy when queue is full.
    pub backpressure: BackpressurePolicy,
    /// Data transfer mode.
    pub transfer_mode: TransferMode,
    /// Boundary classification.
    pub boundary: BoundaryKind,
    /// Timeout in milliseconds. None = unlimited.
    pub timeout_ms: Option<u32>,
    /// Execution priority.
    pub priority: Priority,
    /// Delivery guarantee.
    pub delivery: DeliveryGuarantee,
    /// Observability configuration.
    pub observability: ObservabilitySpec,
}

impl Default for PortSpec {
    fn default() -> Self {
        Self {
            name: "unnamed",
            direction: PortDirection::In,
            queue: QueueKind::Spsc,
            capacity: 1024,
            backpressure: BackpressurePolicy::Block,
            transfer_mode: TransferMode::LoanWrite,
            boundary: BoundaryKind::InterThreadLocal,
            timeout_ms: None,
            priority: Priority::Normal,
            delivery: DeliveryGuarantee::BestEffort,
            observability: ObservabilitySpec::default(),
        }
    }
}

/// Complete specification of a single semantic process.
///
/// A process is the unit of execution and failure domain. Not synonymous
/// with a thread. Can map to a worker thread, async task, or pinned executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessSpec {
    /// Semantic process identifier (e.g. "camera_ingest").
    pub id: &'static str,
    /// Process display name.
    pub name: &'static str,
    /// Execution class.
    pub exec: ExecClass,
    /// Cleanup policy on crash.
    pub cleanup: CleanupPolicy,
    /// Ports owned by this process.
    pub ports: &'static [PortSpec],
    /// Process observability configuration.
    pub observability: ObservabilitySpec,
}

/// Descriptor traveling in queues.
///
/// **Queues carry only descriptors, NOT large payloads.**
/// Contains: sample ID, origin port, lineage ID, region ID.
/// The actual payload lives in the shared exchange heap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Descriptor {
    /// Sample ID on the shared heap.
    pub sample_id: SampleId,
    /// Origin host that published this descriptor.
    pub origin_host: HostId,
    /// Origin port that published this descriptor.
    pub origin_port: PortId,
    /// Lineage ID for end-to-end tracing.
    pub lineage_id: u64,
    /// Timestamp (ns) when the descriptor was published.
    pub publish_ts: u64,
}

/// Static metadata of a message contract.
///
/// Used by the `MessageContract` trait to store declared layout,
/// name, and transfer capability information.
#[derive(Clone, Copy, Debug)]
pub struct MessageMeta {
    /// Message name (e.g. "CameraFrame").
    pub name: &'static str,
    /// Memory layout profile.
    pub layout: LayoutProfile,
    /// Supported transfer modes.
    pub transfer_caps: &'static [TransferMode],
    /// Whether this message is 100% VASI-compliant (contains only POD or VRef).
    /// If false, the runtime must use the Hybrid Data Plane.
    pub is_stable: bool,
    /// Semantic type classification.
    pub semantic_kind: SemanticKind,
    /// Memory class for allocation.
    pub memory_class: MemoryClass,
}

/// Connection metadata for simulated RDMA Send/Write.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnInfo {
    pub host: HostId,
    pub addr: String,
}
