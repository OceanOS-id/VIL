// =============================================================================
// vil_types::enums — Domain Enumerations
// =============================================================================
// Enums defining VIL semantic domains: layout profile, transfer mode,
// queue kind, port direction, boundary, backpressure, execution class,
// cleanup policy, delivery guarantee, priority, and latency class.
//
// All enums derive Copy, Clone, Debug, PartialEq, Eq for zero-overhead
// use as values in specs and IR.
//
// TASK LIST:
// [x] LayoutProfile — flat / relative / external
// [x] TransferMode — loan_write / loan_read / publish_offset / copy / share_read / consume_once
// [x] QueueKind — spsc / mpmc
// [x] PortDirection — in / out / request / response
// [x] BoundaryKind — intra-process to inter-host
// [x] BackpressurePolicy — block / drop_oldest / drop_newest
// [x] ExecClass — thread / async_task / pinned_worker
// [x] CleanupPolicy — reclaim_orphans / leak_on_crash_for_debug
// [x] DeliveryGuarantee — best_effort / at_least_once
// [x] Priority — low / normal / high
// [x] LatencyClass — low / normal / batch
// =============================================================================

use core::fmt;

/// Message layout profile. Determines the memory representation of payloads.
///
/// - `Flat`: Pure POD/VASI, no absolute pointers
/// - `Relative`: Internal references via relative offsets (VRef, VSlice, VStr)
/// - `External`: Non-zero-copy boundary, uses copy/adapter
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LayoutProfile {
    Flat,
    Relative,
    External,
}

/// Data ownership transfer mode between processes.
///
/// - `LoanWrite`: Producer borrows a slot for in-place writing
/// - `LoanRead`: Consumer reads a loan without cloning
/// - `PublishOffset`: Descriptor/offset published to queue
/// - `Copy`: Legal fallback for non-zero-copy boundaries
/// - `ShareRead`: Shared read access to immutable data
/// - `ConsumeOnce`: Linear pattern — resource consumed exactly once
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransferMode {
    LoanWrite,
    LoanRead,
    PublishOffset,
    Copy,
    ShareRead,
    ConsumeOnce,
}

/// Queue kind for descriptor transport.
///
/// - `Spsc`: Single-Producer Single-Consumer (golden path, lock-free target)
/// - `Mpmc`: Multi-Producer Multi-Consumer (opt-in, bounded)
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QueueKind {
    Spsc,
    Mpmc,
}

/// Communication port direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PortDirection {
    In,
    Out,
    Request,
    Response,
}

/// Boundary classification for zero-copy legality.
///
/// Ordered from lightest (intra-process) to heaviest (inter-host).
/// Full zero-copy is only legal for IntraProcess, InterThreadLocal,
/// and InterProcessSharedMemory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BoundaryKind {
    /// Internal boundary: standard borrow, no copy.
    IntraProcess,
    /// Inter-thread within one runtime: queue descriptor + shared object.
    InterThreadLocal,
    /// Inter-process on same host: shared memory + relative offset.
    InterProcessSharedMemory,
    /// Cross-runtime/VM: adapter profile, zero-copy not guaranteed.
    ForeignRuntime,
    /// Cross-host/network: external profile.
    InterHost,
}

/// Backpressure policy when queue is full.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BackpressurePolicy {
    /// Block the producer until a slot is available.
    Block,
    /// Drop the oldest item.
    DropOldest,
    /// Drop the newest (incoming) item.
    DropNewest,
}

/// Process execution class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExecClass {
    /// Dedicated OS thread.
    Thread,
    /// Async task on an executor.
    AsyncTask,
    /// Pinned worker on a specific core.
    PinnedWorker,
}

/// Cleanup policy on process crash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CleanupPolicy {
    /// Reclaim all orphan samples owned by the crashed process.
    ReclaimOrphans,
    /// Leak on crash for post-mortem debugging.
    LeakOnCrashForDebug,
}

/// Message delivery guarantee.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeliveryGuarantee {
    BestEffort,
    AtLeastOnce,
}

/// Execution priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
}

/// Latency class for observability and scheduling hints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LatencyClass {
    Low,
    Normal,
    Batch,
}

// --- Display for key enums ---

impl fmt::Display for LayoutProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat => write!(f, "flat"),
            Self::Relative => write!(f, "relative"),
            Self::External => write!(f, "external"),
        }
    }
}

impl fmt::Display for TransferMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoanWrite => write!(f, "loan_write"),
            Self::LoanRead => write!(f, "loan_read"),
            Self::PublishOffset => write!(f, "publish_offset"),
            Self::Copy => write!(f, "copy"),
            Self::ShareRead => write!(f, "share_read"),
            Self::ConsumeOnce => write!(f, "consume_once"),
        }
    }
}

impl fmt::Display for QueueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spsc => write!(f, "spsc"),
            Self::Mpmc => write!(f, "mpmc"),
        }
    }
}

// =============================================================================
// Core Reactive Primitives
// =============================================================================

/// Lane semantics — role of a port within a reactive interface.
///
/// Each port in a tri-lane reactive interface has a specific role
/// that determines queue topology, priority, and validation rules.
///
/// - `Default`: Standard port without special lane semantics
/// - `Trigger`: Handoff from external ecosystem, defaults to MPMC
/// - `Data`: Hot-path payload, zero-copy friendly
/// - `Control`: DONE/ERROR/ABORT out-of-band, high priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LaneKind {
    /// Standard port without special lane semantics.
    Default,
    /// Trigger lane — receives handoff from external ecosystem.
    /// Default topology: MPMC (multiple external producers).
    Trigger,
    /// Data lane — primary hot-path payload.
    /// Must be free of control signals. Zero-copy friendly.
    Data,
    /// Control lane — session termination signals (DONE, ERROR, ABORT).
    /// Out-of-band, must not be blocked by data payload.
    Control,
}

/// Reactive interface classification.
///
/// Used by compiler and validator to determine applicable
/// validation rules and codegen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReactiveInterfaceKind {
    /// Standard non-reactive interface.
    Normal,
    /// Streaming without session state.
    Streaming,
    /// Session-based reactive — requires tri-lane (trigger + data + control).
    SessionReactive,
}

impl fmt::Display for LaneKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::Trigger => write!(f, "trigger"),
            Self::Data => write!(f, "data"),
            Self::Control => write!(f, "control"),
        }
    }
}

impl fmt::Display for ReactiveInterfaceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Streaming => write!(f, "streaming"),
            Self::SessionReactive => write!(f, "session_reactive"),
        }
    }
}

// =============================================================================
// Semantic Type Classification
// =============================================================================

/// Semantic classification of VIL data types.
///
/// Determines the semantic role of a type in the data plane:
/// - `Message`: General data payload, allowed on all lanes
/// - `State`: State machine data, mutable per-session, Data Lane only
/// - `Event`: Immutable event log, Data Lane or Control Lane
/// - `Fault`: Structured error, Control Lane only
/// - `Decision`: Routing decision, Trigger Lane only
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SemanticKind {
    /// General data payload — allowed on all lanes.
    Message,
    /// State machine data — mutable per-session, Data Lane only.
    State,
    /// Immutable event log entry — Data Lane or Control Lane.
    Event,
    /// Structured error for Control Lane — Control Lane only.
    Fault,
    /// Routing decision payload — Trigger Lane only.
    Decision,
}

impl fmt::Display for SemanticKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message => write!(f, "message"),
            Self::State => write!(f, "state"),
            Self::Event => write!(f, "event"),
            Self::Fault => write!(f, "fault"),
            Self::Decision => write!(f, "decision"),
        }
    }
}

impl SemanticKind {
    /// Returns the set of lanes permitted for this semantic type.
    pub fn allowed_lanes(&self) -> &'static [LaneKind] {
        match self {
            Self::Message => &[
                LaneKind::Default,
                LaneKind::Trigger,
                LaneKind::Data,
                LaneKind::Control,
            ],
            Self::State => &[LaneKind::Data],
            Self::Event => &[LaneKind::Data, LaneKind::Control],
            Self::Fault => &[LaneKind::Control],
            Self::Decision => &[LaneKind::Trigger],
        }
    }

    /// Returns the set of transfer modes permitted for this semantic type.
    pub fn allowed_transfer_modes(&self) -> &'static [TransferMode] {
        match self {
            Self::Message => &[
                TransferMode::LoanWrite,
                TransferMode::LoanRead,
                TransferMode::Copy,
            ],
            Self::State => &[TransferMode::LoanWrite, TransferMode::LoanRead],
            Self::Event => &[TransferMode::LoanWrite, TransferMode::Copy],
            Self::Fault => &[TransferMode::Copy],
            Self::Decision => &[TransferMode::LoanWrite, TransferMode::Copy],
        }
    }
}

/// Memory class for VIL data types.
///
/// Determines the applicable allocation and transfer strategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MemoryClass {
    /// Paged exchange heap — default allocation for Data Lane.
    PagedExchange,
    /// Pinned remote-ready memory — for RDMA/hardware DMA.
    PinnedRemote,
    /// Control heap — lightweight allocation for control signals.
    ControlHeap,
    /// Local scratch arena — temporary per-process.
    LocalScratch,
}

impl fmt::Display for MemoryClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PagedExchange => write!(f, "paged_exchange"),
            Self::PinnedRemote => write!(f, "pinned_remote"),
            Self::ControlHeap => write!(f, "control_heap"),
            Self::LocalScratch => write!(f, "local_scratch"),
        }
    }
}

impl MemoryClass {
    /// Compatibility matrix: which TransferModes are legal for this MemoryClass.
    ///
    /// | MemoryClass    | LoanWrite | LoanRead | Copy | RemotePull (PublishOffset) |
    /// |----------------|-----------|----------|------|---------------------------|
    /// | PagedExchange  | ✅        | ✅       | ❌   | ❌                        |
    /// | PinnedRemote   | ✅        | ✅       | ❌   | ✅                        |
    /// | ControlHeap    | ❌        | ❌       | ✅   | ❌                        |
    /// | LocalScratch   | ✅        | ✅       | ✅   | ❌                        |
    pub const fn allowed_transfer_modes(&self) -> &'static [TransferMode] {
        match self {
            Self::PagedExchange => &[TransferMode::LoanWrite, TransferMode::LoanRead],
            Self::PinnedRemote => &[
                TransferMode::LoanWrite,
                TransferMode::LoanRead,
                TransferMode::PublishOffset,
            ],
            Self::ControlHeap => &[TransferMode::Copy],
            Self::LocalScratch => &[
                TransferMode::LoanWrite,
                TransferMode::LoanRead,
                TransferMode::Copy,
            ],
        }
    }

    /// Human-readable description for error messages and dashboards.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::PagedExchange => "Paged exchange heap — zero-copy Data Lane",
            Self::PinnedRemote => "Pinned remote-ready memory — RDMA/DMA capable",
            Self::ControlHeap => "Control heap — small copy-only control signals",
            Self::LocalScratch => "Local scratch arena — temporary per-process",
        }
    }
}

// =============================================================================
// Trust Zone & Capsule System
// =============================================================================

/// Process execution zone in the VIL system.
/// Determines the trust level and capability restrictions for a process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TrustZone {
    /// Kernel/core internal, full privileges. Only first-party trusted code.
    NativeCore,
    /// Trusted native process. Nearly all capabilities except secrets.
    NativeTrusted,
    /// WASM plugin in sandbox. Limited capabilities, no shared memory.
    WasmCapsule,
    /// External third-party adapter. Minimal privileges.
    ExternalBoundary,
}

impl fmt::Display for TrustZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NativeCore => write!(f, "NativeCore"),
            Self::NativeTrusted => write!(f, "NativeTrusted"),
            Self::WasmCapsule => write!(f, "WasmCapsule"),
            Self::ExternalBoundary => write!(f, "ExternalBoundary"),
        }
    }
}

/// Capabilities granted to a Trust Zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ZoneCapability {
    /// Can emit events/messages to Data or Trigger Lane.
    CanEmitLane,
    /// Can read state from shared-memory or registry.
    CanReadState,
    /// Can access credentials/secrets from vault.
    CanUseSecret,
    /// Can allocate and access shared-memory regions.
    CanAccessShm,
    /// Can join the cluster node registry.
    CanJoinCluster,
    /// Can open remote network connections (RDMA, TCP).
    CanUseRemote,
}

/// Capability table per trust zone.
pub const fn zone_capabilities(zone: TrustZone) -> &'static [ZoneCapability] {
    use ZoneCapability::*;
    match zone {
        TrustZone::NativeCore => &[
            CanEmitLane,
            CanReadState,
            CanUseSecret,
            CanAccessShm,
            CanJoinCluster,
            CanUseRemote,
        ],
        TrustZone::NativeTrusted => &[
            CanEmitLane,
            CanReadState,
            CanAccessShm,
            CanJoinCluster,
            CanUseRemote,
        ],
        TrustZone::WasmCapsule => &[CanEmitLane],
        TrustZone::ExternalBoundary => &[],
    }
}

// =============================================================================
// Semantic Activity System
// =============================================================================

/// Activity kind classification within VIL pipelines.
///
/// An activity is a semantic work unit executed inside a pipeline.
/// Each kind has different input/output contracts and zone constraints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ActivityKind {
    /// Native Rust logic — full substrate access, maximum performance.
    Native,
    /// Logic in WASM Capsule — sandboxed, limited capabilities.
    Capsule,
    /// Stateless payload transformation — no side effects.
    Transform,
    /// Condition-based rule evaluation — decision engine.
    Rule,
    /// Integration adapter to external systems — Kafka, gRPC, REST, etc.
    Connector,
}

impl fmt::Display for ActivityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native => write!(f, "native"),
            Self::Capsule => write!(f, "capsule"),
            Self::Transform => write!(f, "transform"),
            Self::Rule => write!(f, "rule"),
            Self::Connector => write!(f, "connector"),
        }
    }
}

impl ActivityKind {
    /// Minimum trust zone permitted for this activity kind.
    pub fn minimum_zone(&self) -> TrustZone {
        match self {
            Self::Native => TrustZone::NativeTrusted,
            Self::Capsule => TrustZone::WasmCapsule,
            Self::Transform => TrustZone::WasmCapsule,
            Self::Rule => TrustZone::WasmCapsule,
            Self::Connector => TrustZone::NativeTrusted,
        }
    }

    /// Whether this activity kind is allowed to run in the given zone.
    pub fn is_allowed_in_zone(&self, zone: TrustZone) -> bool {
        match self {
            Self::Native => matches!(zone, TrustZone::NativeCore | TrustZone::NativeTrusted),
            Self::Capsule => matches!(zone, TrustZone::WasmCapsule),
            Self::Transform => true, // Stateless — allowed anywhere
            Self::Rule => !matches!(zone, TrustZone::ExternalBoundary),
            Self::Connector => matches!(zone, TrustZone::NativeCore | TrustZone::NativeTrusted),
        }
    }
}

/// Lifecycle state of an activity instance.
///
/// ```text
/// Idle → Activated → Running → Completed / Faulted → Deactivated
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ActivityLifecycle {
    /// Not yet activated, definition only.
    Idle,
    /// Registered and ready to receive input.
    Activated,
    /// Currently executing logic.
    Running,
    /// Execution completed successfully.
    Completed,
    /// Execution failed — fault emitted to Control Lane.
    Faulted,
    /// Activity deactivated and cleaned up.
    Deactivated,
}

impl fmt::Display for ActivityLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Activated => write!(f, "activated"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Faulted => write!(f, "faulted"),
            Self::Deactivated => write!(f, "deactivated"),
        }
    }
}

impl ActivityLifecycle {
    /// Valid transitions from this state.
    pub fn valid_transitions(&self) -> &'static [ActivityLifecycle] {
        match self {
            Self::Idle => &[Self::Activated],
            Self::Activated => &[Self::Running, Self::Deactivated],
            Self::Running => &[Self::Completed, Self::Faulted],
            Self::Completed => &[Self::Deactivated, Self::Activated], // re-activation
            Self::Faulted => &[Self::Deactivated, Self::Activated],   // retry
            Self::Deactivated => &[],
        }
    }

    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Deactivated)
    }
}
