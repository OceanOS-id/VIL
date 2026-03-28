// =============================================================================
// vil_ir::core — Canonical Semantic IR Nodes
// =============================================================================
// Internal AST representation of the VIL system. This is the convergence
// point for all authoring forms (Dot Builder, Macro) before validation
// and code generation to the Rust runtime substrate (`vil_rt`).
//
// All structures are 100% Rust-native, focused on the internal compiler
// pipeline.
// =============================================================================

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vil_types::{
    ActivityKind, BackpressurePolicy, BoundaryKind, CleanupPolicy, DeliveryGuarantee, ExecClass,
    LaneKind, LatencyClass, LayoutProfile, PortDirection, Priority, QueueKind,
    ReactiveInterfaceKind, TransferMode,
};

/// Root of the Semantic IR. Represents a complete pipeline/graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowIR {
    pub name: String,
    pub processes: HashMap<String, ProcessIR>,
    pub interfaces: HashMap<String, InterfaceIR>,
    pub messages: HashMap<String, MessageIR>,
    /// Routes between ports: from_process.port -> to_process.port
    pub routes: Vec<RouteIR>,
    /// Transfer expressions (first-class transfer semantics).
    pub transfers: Vec<TransferExprIR>,
    /// Ownership transition records for lifecycle tracking.
    pub ownership_transitions: Vec<OwnershipTransitionIR>,
    /// Host topology for distributed routing
    pub hosts: HashMap<String, HostIR>,
    /// High-availability failover definitions
    pub failovers: Vec<FailoverIR>,
    /// Semantic activities within the pipeline
    pub activities: HashMap<String, ActivityIR>,
}

impl WorkflowIR {
    /// Extract all accumulated metadata into a serialisable `ExecutionContract`.
    pub fn to_contract(&self) -> crate::contract::ExecutionContract {
        use crate::contract::*;
        use vil_types::{zone_capabilities, ZoneCapability};

        // --- Trust profile: union of capabilities across all processes ---
        let mut trust_profile = TrustProfile::default();
        let mut execution_zone = "Unset".to_string();
        let mut found_zone = false;

        for proc in self.processes.values() {
            if let Some(zone) = proc.trust_zone {
                if !found_zone {
                    execution_zone = format!("{}", zone);
                    found_zone = true;
                }
                let caps = zone_capabilities(zone);
                if caps.contains(&ZoneCapability::CanEmitLane) {
                    trust_profile.can_emit_lane = true;
                }
                if caps.contains(&ZoneCapability::CanReadState) {
                    trust_profile.can_read_state = true;
                }
                if caps.contains(&ZoneCapability::CanUseSecret) {
                    trust_profile.can_use_secret = true;
                }
                if caps.contains(&ZoneCapability::CanAccessShm) {
                    trust_profile.can_access_shm = true;
                }
                if caps.contains(&ZoneCapability::CanJoinCluster) {
                    trust_profile.can_join_cluster = true;
                }
                if caps.contains(&ZoneCapability::CanUseRemote) {
                    trust_profile.can_use_remote = true;
                }
            }
        }

        // --- Lanes: one LaneEntry per route ---
        let lanes: Vec<LaneEntry> = self
            .routes
            .iter()
            .map(|route| {
                let (lane_kind, memory_class) = self
                    .interfaces
                    .values()
                    .find_map(|iface| {
                        iface.ports.get(&route.from_port).map(|port| {
                            let mc = self
                                .messages
                                .get(&port.message_name)
                                .map(|m| format!("{}", m.memory_class))
                                .unwrap_or_else(|| "unknown".into());
                            (format!("{}", port.lane_kind), mc)
                        })
                    })
                    .unwrap_or_else(|| ("default".into(), "unknown".into()));

                LaneEntry {
                    route: format!(
                        "{}.{} -> {}.{}",
                        route.from_process, route.from_port, route.to_process, route.to_port
                    ),
                    lane_kind,
                    transfer: format!("{}", route.transfer_mode),
                    memory_class,
                }
            })
            .collect();

        // --- Hosts ---
        let mut hosts: Vec<String> = self.hosts.values().map(|h| h.address.clone()).collect();
        hosts.sort();

        // --- Failovers ---
        let failover: Vec<FailoverEntry> = self
            .failovers
            .iter()
            .map(|f| FailoverEntry {
                source: f.source.clone(),
                target: f.target.clone(),
                condition: f.condition.clone(),
                strategy: f.strategy.clone(),
            })
            .collect();

        // --- Observability: aggregate across processes ---
        let mut trace_hops = false;
        let mut latency_markers: Vec<String> = Vec::new();
        for proc in self.processes.values() {
            if proc.obs.trace_hop {
                trace_hops = true;
            }
            if let Some(ref label) = proc.obs.latency_label {
                if !latency_markers.contains(label) {
                    latency_markers.push(label.clone());
                }
            }
        }
        latency_markers.sort();

        // --- Process summaries ---
        let mut processes: Vec<ProcessSummary> = self
            .processes
            .values()
            .map(|p| {
                ProcessSummary {
                    name: p.name.clone(),
                    exec_class: format!("{:?}", p.exec_class).to_lowercase(),
                    trust_zone: p.trust_zone.map(|z| format!("{}", z)),
                    host_affinity: p.host_affinity.clone(),
                    trace_hop: p.obs.trace_hop,
                    latency_label: p.obs.latency_label.clone(),
                    memory_class: None, // future: resolve from output port message
                }
            })
            .collect();
        processes.sort_by(|a, b| a.name.cmp(&b.name));

        ExecutionContract {
            pipeline: self.name.clone(),
            execution_zone,
            trust_profile,
            lanes,
            hosts,
            failover,
            observability: ObservabilityEntry {
                trace_hops,
                latency_markers,
            },
            processes,
        }
    }

    /// Serialise this workflow's Execution Contract to a pretty-printed JSON string.
    pub fn to_contract_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.to_contract())
    }

    /// Serialise this workflow's Execution Contract to a YAML string.
    /// Compatible with the .vwfd workflow file format style.
    pub fn to_contract_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(&self.to_contract())
    }
}

/// Represents high-availability failover intentions in a workflow.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailoverIR {
    /// The process that can fail
    pub source: String,

    /// The target process taking over OR a retry strategy representation.
    pub target: String,

    /// The condition (`#[vil_fault]`) causing this failover.
    pub condition: String,

    /// String formatted failover strategy.
    pub strategy: String,
}

/// Represents a node (host) in the distributed topology.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostIR {
    pub name: String,
    pub address: String, // e.g. "192.168.1.10:9000"
}

/// Represents a semantic activity within the pipeline.
///
/// An activity is a semantic work unit with validated input/output/fault
/// contracts. Each ActivityIR is included in the ExecutionContract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityIR {
    pub name: String,
    pub kind: ActivityKind,
    /// Accepted input type (must exist in `messages`).
    pub input_type: Option<String>,
    /// Produced output type (must exist in `messages`).
    pub output_type: Option<String>,
    /// Possible fault type (must exist in `messages`).
    pub fault_type: Option<String>,
    /// Trust zone requirement for this activity.
    pub zone: Option<vil_types::TrustZone>,
    /// Observability annotation.
    pub obs: ObsIR,
}

/// Observability annotations on a process.
/// Emitted by `#[trace_hop]` and `#[latency_marker("label")]`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ObsIR {
    /// If true, record hop latency every time a message crosses this process.
    pub trace_hop: bool,
    /// Optional named label for dashboarding (from `#[latency_marker("label")]`).
    pub latency_label: Option<String>,
}

/// Represents a Process definition.
/// A process is a failure domain and unit of execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessIR {
    pub name: String,
    pub interface_name: String,
    pub exec_class: ExecClass,
    pub cleanup_policy: CleanupPolicy,
    pub priority: Priority,
    /// Host placement constraint
    pub host_affinity: Option<String>,
    /// Trust execution zone
    pub trust_zone: Option<vil_types::TrustZone>,
    /// Observability annotations
    pub obs: ObsIR,
}

impl ProcessIR {
    pub fn host_affinity(mut self, host: impl Into<String>) -> Self {
        self.host_affinity = Some(host.into());
        self
    }
    pub fn trust_zone(mut self, zone: vil_types::TrustZone) -> Self {
        self.trust_zone = Some(zone);
        self
    }
    pub fn obs_trace_hop(mut self) -> Self {
        self.obs.trace_hop = true;
        self
    }
    pub fn obs_latency_label(mut self, label: impl Into<String>) -> Self {
        self.obs.latency_label = Some(label.into());
        self
    }
}

/// Represents an interface contract (collection of Ports) implementable by a Process.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterfaceIR {
    pub name: String,
    pub ports: HashMap<String, PortIR>,
    /// Reactive interface classification.
    pub reactive_kind: ReactiveInterfaceKind,
    /// Host placement constraint (for instances)
    pub host_affinity: Option<String>,
    /// Trust execution zone
    pub trust_zone: Option<vil_types::TrustZone>,
}

impl InterfaceIR {
    pub fn host_affinity(mut self, host: impl Into<String>) -> Self {
        self.host_affinity = Some(host.into());
        self
    }
    pub fn trust_zone(mut self, zone: vil_types::TrustZone) -> Self {
        self.trust_zone = Some(zone);
        self
    }
}

/// Represents a single entry/exit point on an interface.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortIR {
    pub name: String,
    pub direction: PortDirection,
    pub message_name: String,
    pub queue_spec: QueueIR,
    pub timeout_ms: Option<u64>,
    /// Semantic lane role for this port.
    pub lane_kind: LaneKind,
}

/// Queue infrastructure specification backing a port.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueIR {
    pub kind: QueueKind,
    pub capacity: usize,
    pub backpressure: BackpressurePolicy,
}

/// Represents a payload contract (Message/Event).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageIR {
    pub name: String,
    pub layout: LayoutProfile,
    pub delivery: DeliveryGuarantee,
    pub latency_class: LatencyClass,
    pub fields: Vec<FieldIR>,
    /// Semantic type classification.
    pub semantic_kind: vil_types::SemanticKind,
    /// Memory class for allocation.
    pub memory_class: vil_types::MemoryClass,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldIR {
    pub name: String,
    pub ty: TypeRefIR,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeRefIR {
    Primitive(String), // e.g., "u32", "i64"
    Named(String),     // Other struct name
    VSlice(Box<TypeRefIR>),
    VRef(Box<TypeRefIR>),
    Unknown(String), // Handled as potential VASI but unverified
}

#[cfg(feature = "proc-macro")]
impl quote::ToTokens for TypeRefIR {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        use quote::quote;
        let expanded = match self {
            TypeRefIR::Primitive(s) => {
                quote! { ::vil_sdk::ir::TypeRefIR::Primitive(#s.to_string()) }
            }
            TypeRefIR::Named(s) => quote! { ::vil_sdk::ir::TypeRefIR::Named(#s.to_string()) },
            TypeRefIR::VSlice(inner) => {
                quote! { ::vil_sdk::ir::TypeRefIR::VSlice(Box::new(#inner)) }
            }
            TypeRefIR::VRef(inner) => quote! { ::vil_sdk::ir::TypeRefIR::VRef(Box::new(#inner)) },
            TypeRefIR::Unknown(s) => quote! { ::vil_sdk::ir::TypeRefIR::Unknown(#s.to_string()) },
        };
        tokens.extend(expanded);
    }
}

/// Defines the data flow topology between ports.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteIR {
    pub from_process: String,
    pub from_port: String,
    pub to_process: String,
    pub to_port: String,
    pub transfer_mode: TransferMode,
    pub boundary: BoundaryKind, // Auto-determined by the validator
    /// Physical message distribution scope
    pub scope: RouteScope,
    /// Transport plugin layer hint (e.g. RDMA, TCP)
    pub transport: Option<String>,
}

/// Route scope category (Local vs Distributed)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteScope {
    /// Within a single OS process (thread-to-thread)
    Local,
    /// Between OS processes on the same host (shared memory IPC)
    SharedMemory,
    /// Across hosts, requires network transport (vil_net)
    Remote,
}

// =============================================================================
// Lifecycle & Transfer DSL
// =============================================================================

/// Ownership lifecycle state for each message/sample.
///
/// Each message passes through a linear state machine:
/// `Allocated -> Published -> Received -> Released -> Reclaimed`
///
/// The validator can detect leaks if a message is Published but never Released.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnershipState {
    /// Slot allocated from the exchange heap, not yet written.
    Allocated,
    /// Data written and descriptor published to the queue.
    Published,
    /// Consumer received the descriptor and read the data.
    Received,
    /// Consumer finished reading, ownership returned to the pool.
    Released,
    /// Runtime reclaimed the slot — lifecycle complete.
    Reclaimed,
}

impl std::fmt::Display for OwnershipState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allocated => write!(f, "allocated"),
            Self::Published => write!(f, "published"),
            Self::Received => write!(f, "received"),
            Self::Released => write!(f, "released"),
            Self::Reclaimed => write!(f, "reclaimed"),
        }
    }
}

impl OwnershipState {
    /// Returns the next valid state in the lifecycle.
    pub fn next_valid(&self) -> Option<OwnershipState> {
        match self {
            Self::Allocated => Some(Self::Published),
            Self::Published => Some(Self::Received),
            Self::Received => Some(Self::Released),
            Self::Released => Some(Self::Reclaimed),
            Self::Reclaimed => None, // terminal state
        }
    }

    /// Whether the transition from the current state to the target is valid.
    pub fn can_transition_to(&self, target: &OwnershipState) -> bool {
        self.next_valid().as_ref() == Some(target)
    }

    /// Whether this state is terminal (no outgoing transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Reclaimed)
    }
}

/// Transfer expression — first-class representation of transfer semantics.
///
/// Replaces manual API calls (allocate -> write -> publish)
/// with declarative expressions understood by the compiler.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferExprIR {
    /// Unique name of the transfer expression.
    pub name: String,
    /// Source process.port that sends.
    pub from_process: String,
    pub from_port: String,
    /// Destination process.port that receives.
    pub to_process: String,
    pub to_port: String,
    /// Transfer mode (determined by the route, not in code).
    pub transfer_mode: TransferMode,
    /// Name of the message type being transferred.
    pub message_name: String,
    /// Expected ownership flow for this transfer.
    pub expected_flow: Vec<OwnershipState>,
}

/// Record of ownership transitions per message in the lifecycle.
///
/// Used by the validator to detect memory leaks
/// (Published but never Released).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OwnershipTransitionIR {
    /// Tracked message name.
    pub message_name: String,
    /// Process performing the transition.
    pub process_name: String,
    /// State before the transition.
    pub from_state: OwnershipState,
    /// State after the transition.
    pub to_state: OwnershipState,
    /// Associated port (if any).
    pub port_name: Option<String>,
}

impl WorkflowIR {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            processes: HashMap::new(),
            interfaces: HashMap::new(),
            messages: HashMap::new(),
            routes: Vec::new(),
            transfers: Vec::new(),
            ownership_transitions: Vec::new(),
            hosts: HashMap::new(),
            failovers: Vec::new(),
            activities: HashMap::new(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
