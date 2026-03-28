// =============================================================================
// vil_ir::builder — Dot Builder API
// =============================================================================
// Fluent interface for building the Semantic IR programmatically.
// This API serves as the foundation for Rust-based authoring and is
// wrapped by VIL Macros.
// =============================================================================

use crate::core::{InterfaceIR, MessageIR, PortIR, ProcessIR, QueueIR, RouteIR, WorkflowIR};
use vil_types::{
    BackpressurePolicy, BoundaryKind, CleanupPolicy, DeliveryGuarantee, ExecClass, LaneKind,
    LatencyClass, LayoutProfile, PortDirection, Priority, QueueKind, ReactiveInterfaceKind,
    TransferMode,
};

/// Primary builder for assembling a complete `WorkflowIR`.
pub struct WorkflowBuilder {
    ir: WorkflowIR,
}

impl WorkflowBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            ir: WorkflowIR {
                name: name.into(),
                processes: std::collections::HashMap::new(),
                interfaces: std::collections::HashMap::new(),
                messages: std::collections::HashMap::new(),
                routes: Vec::new(),
                transfers: Vec::new(),
                ownership_transitions: Vec::new(),
                hosts: std::collections::HashMap::new(),
                failovers: Vec::new(),
                activities: std::collections::HashMap::new(),
            },
        }
    }

    pub fn add_message(mut self, msg: MessageIR) -> Self {
        self.ir.messages.insert(msg.name.clone(), msg);
        self
    }

    /// Add a host to the topology
    pub fn add_host(mut self, name: impl Into<String>, address: impl Into<String>) -> Self {
        let n: String = name.into();
        self.ir.hosts.insert(
            n.clone(),
            crate::core::HostIR {
                name: n,
                address: address.into(),
            },
        );
        self
    }

    /// Add a high-availability failover specification
    pub fn failover(
        mut self,
        source: impl Into<String>,
        target: impl Into<String>,
        condition: impl Into<String>,
        strategy: impl Into<String>,
    ) -> Self {
        self.ir.failovers.push(crate::core::FailoverIR {
            source: source.into(),
            target: target.into(),
            condition: condition.into(),
            strategy: strategy.into(),
        });
        self
    }

    pub fn add_interface(mut self, iface: InterfaceIR) -> Self {
        self.ir.interfaces.insert(iface.name.clone(), iface);
        self
    }

    pub fn add_process(mut self, proc: ProcessIR) -> Self {
        self.ir.processes.insert(proc.name.clone(), proc);
        self
    }

    pub fn route(
        self,
        from_process: impl Into<String>,
        from_port: impl Into<String>,
        to_process: impl Into<String>,
        to_port: impl Into<String>,
        transfer_mode: TransferMode,
    ) -> Self {
        self.route_ext(
            from_process,
            from_port,
            to_process,
            to_port,
            transfer_mode,
            None,
        )
    }

    pub fn route_ext(
        mut self,
        from_process: impl Into<String>,
        from_port: impl Into<String>,
        to_process: impl Into<String>,
        to_port: impl Into<String>,
        transfer_mode: TransferMode,
        transport: Option<String>,
    ) -> Self {
        let from_process = from_process.into();
        let to_process = to_process.into();

        // Auto-resolve Scope
        let mut scope = crate::core::RouteScope::Local;
        if let (Some(fp), Some(tp)) = (
            self.ir.processes.get(&from_process),
            self.ir.processes.get(&to_process),
        ) {
            match (&fp.host_affinity, &tp.host_affinity) {
                (Some(fh), Some(th)) if fh != th => {
                    scope = crate::core::RouteScope::Remote;
                }
                _ => {}
            }
        }

        self.ir.routes.push(RouteIR {
            from_process,
            from_port: from_port.into(),
            to_process,
            to_port: to_port.into(),
            transfer_mode,
            boundary: BoundaryKind::IntraProcess, // Default, will be updated by ValidatePass
            scope,
            transport,
        });
        self
    }

    pub fn build(self) -> WorkflowIR {
        self.ir
    }

    /// Infer TransferExprIR from existing routes.
    ///
    /// For each route, creates a transfer expression with the expected
    /// ownership flow matching the transfer mode.
    pub fn infer_transfers(mut self) -> Self {
        let mut transfers = Vec::new();
        for (i, route) in self.ir.routes.iter().enumerate() {
            // Find message name from the source port
            let msg_name = self
                .ir
                .interfaces
                .values()
                .flat_map(|iface| iface.ports.values())
                .find(|p| p.name == route.from_port)
                .map(|p| p.message_name.clone())
                .unwrap_or_else(|| format!("unknown_{}", i));

            let expected_flow = vec![
                crate::core::OwnershipState::Allocated,
                crate::core::OwnershipState::Published,
                crate::core::OwnershipState::Received,
                crate::core::OwnershipState::Released,
                crate::core::OwnershipState::Reclaimed,
            ];

            transfers.push(crate::core::TransferExprIR {
                name: format!(
                    "transfer_{}_{}_to_{}_{}",
                    route.from_process, route.from_port, route.to_process, route.to_port
                ),
                from_process: route.from_process.clone(),
                from_port: route.from_port.clone(),
                to_process: route.to_process.clone(),
                to_port: route.to_port.clone(),
                transfer_mode: route.transfer_mode,
                message_name: msg_name,
                expected_flow,
            });
        }
        self.ir.transfers = transfers;
        self
    }
}

/// Builder for `MessageIR`.
pub struct MessageBuilder {
    ir: MessageIR,
}

impl MessageBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            ir: MessageIR {
                name: name.into(),
                layout: LayoutProfile::Relative, // Safe default for VIL zero-copy
                delivery: DeliveryGuarantee::AtLeastOnce,
                latency_class: LatencyClass::Normal,
                fields: Vec::new(),
                semantic_kind: vil_types::SemanticKind::Message,
                memory_class: vil_types::MemoryClass::PagedExchange,
            },
        }
    }

    pub fn layout(mut self, layout: LayoutProfile) -> Self {
        self.ir.layout = layout;
        self
    }

    pub fn delivery(mut self, delivery: DeliveryGuarantee) -> Self {
        self.ir.delivery = delivery;
        self
    }

    pub fn latency(mut self, latency: LatencyClass) -> Self {
        self.ir.latency_class = latency;
        self
    }

    pub fn semantic_kind(mut self, kind: vil_types::SemanticKind) -> Self {
        self.ir.semantic_kind = kind;
        self
    }

    pub fn memory_class(mut self, class: vil_types::MemoryClass) -> Self {
        self.ir.memory_class = class;
        self
    }

    pub fn add_field(mut self, name: impl Into<String>, ty: crate::core::TypeRefIR) -> Self {
        self.ir.fields.push(crate::core::FieldIR {
            name: name.into(),
            ty,
        });
        self
    }

    pub fn build(self) -> MessageIR {
        self.ir
    }
}

/// Builder for `InterfaceIR`.
pub struct InterfaceBuilder {
    ir: InterfaceIR,
}

impl InterfaceBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            ir: InterfaceIR {
                name: name.into(),
                ports: std::collections::HashMap::new(),
                reactive_kind: ReactiveInterfaceKind::Normal,
                host_affinity: None,
                trust_zone: None,
            },
        }
    }

    pub fn host_affinity(mut self, host: impl Into<String>) -> Self {
        self.ir.host_affinity = Some(host.into());
        self
    }

    pub fn trust_zone(mut self, zone: vil_types::TrustZone) -> Self {
        self.ir.trust_zone = Some(zone);
        self
    }

    pub fn in_port(self, name: impl Into<String>, message_name: impl Into<String>) -> PortBuilder {
        PortBuilder::new(self, name, PortDirection::In, message_name)
    }

    pub fn out_port(self, name: impl Into<String>, message_name: impl Into<String>) -> PortBuilder {
        PortBuilder::new(self, name, PortDirection::Out, message_name)
    }

    // Called internally by PortBuilder
    fn add_port(&mut self, port: PortIR) {
        self.ir.ports.insert(port.name.clone(), port);
    }

    pub fn build(self) -> InterfaceIR {
        self.ir
    }
}

/// Intermediate builder for configuring `PortIR`, then returning to `InterfaceBuilder`.
pub struct PortBuilder {
    parent: InterfaceBuilder,
    ir: PortIR,
}

impl PortBuilder {
    fn new(
        parent: InterfaceBuilder,
        name: impl Into<String>,
        direction: PortDirection,
        message_name: impl Into<String>,
    ) -> Self {
        Self {
            parent,
            ir: PortIR {
                name: name.into(),
                direction,
                message_name: message_name.into(),
                queue_spec: QueueIR {
                    kind: QueueKind::Spsc, // Golden-path VIL
                    capacity: 1024,
                    backpressure: BackpressurePolicy::DropOldest,
                },
                timeout_ms: None,
                lane_kind: LaneKind::Default,
            },
        }
    }

    pub fn queue(mut self, kind: QueueKind, capacity: usize) -> Self {
        self.ir.queue_spec.kind = kind;
        self.ir.queue_spec.capacity = capacity;
        self
    }

    pub fn backpressure(mut self, policy: BackpressurePolicy) -> Self {
        self.ir.queue_spec.backpressure = policy;
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.ir.timeout_ms = Some(ms);
        self
    }

    /// Set lane semantics for this port.
    pub fn lane(mut self, kind: LaneKind) -> Self {
        self.ir.lane_kind = kind;
        self
    }

    /// Finalize this port and return to chaining on InterfaceBuilder.
    pub fn done(mut self) -> InterfaceBuilder {
        self.parent.add_port(self.ir);
        self.parent
    }
}

/// Builder for `ProcessIR`.
pub struct ProcessBuilder {
    ir: ProcessIR,
}

impl ProcessBuilder {
    pub fn new(name: impl Into<String>, interface_name: impl Into<String>) -> Self {
        Self {
            ir: ProcessIR {
                name: name.into(),
                interface_name: interface_name.into(),
                exec_class: ExecClass::Thread,
                cleanup_policy: CleanupPolicy::ReclaimOrphans,
                priority: Priority::Normal,
                host_affinity: None,
                trust_zone: None,
                obs: crate::core::ObsIR::default(),
            },
        }
    }

    pub fn host_affinity(mut self, host: impl Into<String>) -> Self {
        self.ir.host_affinity = Some(host.into());
        self
    }

    pub fn trust_zone(mut self, zone: vil_types::TrustZone) -> Self {
        self.ir.trust_zone = Some(zone);
        self
    }

    pub fn obs_trace_hop(mut self) -> Self {
        self.ir.obs.trace_hop = true;
        self
    }

    pub fn obs_latency_label(mut self, label: impl Into<String>) -> Self {
        self.ir.obs.latency_label = Some(label.into());
        self
    }

    pub fn exec_class(mut self, class: ExecClass) -> Self {
        self.ir.exec_class = class;
        self
    }

    pub fn cleanup(mut self, policy: CleanupPolicy) -> Self {
        self.ir.cleanup_policy = policy;
        self
    }

    pub fn priority(mut self, p: Priority) -> Self {
        self.ir.priority = p;
        self
    }

    pub fn build(self) -> ProcessIR {
        self.ir
    }
}
