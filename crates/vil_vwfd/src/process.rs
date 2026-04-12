//! VwfdKernel — ProcessSpec definition for VIL VWFD.
//!
//! Defines the VIL process specification for VWFD workflow execution.
//! ProcessSpec is used when wiring via vil_workflow! macro (SHM pipeline mode).
//!
//! Current serve() uses VilApp ServiceProcess (HTTP mode).
//! Pipeline mode (HttpSink → VwfdKernel → HttpSink) available for future use
//! when vil_new_http HttpSink supports wildcard path matching.

use vil_types::{
    BackpressurePolicy, BoundaryKind, CleanupPolicy, DeliveryGuarantee, ExecClass,
    ObservabilitySpec, PortDirection, PortSpec, Priority, ProcessSpec,
    QueueKind, TransferMode,
};

/// Static port definitions for VWFD kernel process.
static VWFD_PORTS: &[PortSpec] = &[
    PortSpec {
        name: "trigger_in",
        direction: PortDirection::In,
        queue: QueueKind::Spsc,
        capacity: 256,
        backpressure: BackpressurePolicy::Block,
        transfer_mode: TransferMode::LoanWrite,
        boundary: BoundaryKind::InterThreadLocal,
        timeout_ms: None,
        priority: Priority::Normal,
        delivery: DeliveryGuarantee::BestEffort,
        observability: ObservabilitySpec {
            tracing: true, metrics: true, lineage: true,
            audit_sample_handoff: false,
            latency_class: vil_types::LatencyClass::Normal,
        },
    },
    PortSpec {
        name: "data_out",
        direction: PortDirection::Out,
        queue: QueueKind::Spsc,
        capacity: 256,
        backpressure: BackpressurePolicy::Block,
        transfer_mode: TransferMode::LoanWrite,
        boundary: BoundaryKind::InterThreadLocal,
        timeout_ms: None,
        priority: Priority::Normal,
        delivery: DeliveryGuarantee::BestEffort,
        observability: ObservabilitySpec {
            tracing: true, metrics: true, lineage: true,
            audit_sample_handoff: false,
            latency_class: vil_types::LatencyClass::Normal,
        },
    },
];

/// VWFD kernel ProcessSpec — VIL runtime registration.
pub static VWFD_PROCESS_SPEC: ProcessSpec = ProcessSpec {
    id: "vil_vwfd_kernel",
    name: "VWFD Kernel",
    exec: ExecClass::AsyncTask,
    cleanup: CleanupPolicy::ReclaimOrphans,
    ports: VWFD_PORTS,
    observability: ObservabilitySpec {
        tracing: true, metrics: true, lineage: true,
        audit_sample_handoff: false,
        latency_class: vil_types::LatencyClass::Normal,
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_spec() {
        assert_eq!(VWFD_PROCESS_SPEC.id, "vil_vwfd_kernel");
        assert_eq!(VWFD_PROCESS_SPEC.ports.len(), 2);
        assert_eq!(VWFD_PROCESS_SPEC.ports[0].name, "trigger_in");
        assert_eq!(VWFD_PROCESS_SPEC.ports[1].name, "data_out");
    }
}
