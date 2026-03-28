// =============================================================================
// vil_ir — Canonical Semantic IR
// =============================================================================
// Single source of truth for the VIL system. Internal AST representation
// of Workflow, Process, Interface, Message, and Port nodes.
//
// Structures represent applicative intent before validation and compilation
// into the Rust zero-copy runtime substrate.
//
// A fluent `builder` API (Dot Builder) is provided for structured,
// type-state IR assembly.
// =============================================================================

pub mod builder;
pub mod contract;
pub mod core;

pub use builder::{InterfaceBuilder, MessageBuilder, PortBuilder, ProcessBuilder, WorkflowBuilder};
pub use contract::{
    ExecutionContract, FailoverEntry, LaneEntry, ObservabilityEntry, ProcessSummary, TrustProfile,
};
pub use core::{
    FieldIR, InterfaceIR, MessageIR, OwnershipState, OwnershipTransitionIR, PortIR, ProcessIR,
    QueueIR, RouteIR, TransferExprIR, TypeRefIR, WorkflowIR,
};

#[cfg(test)]
mod tests {
    use super::*;
    use vil_types::{BackpressurePolicy, ExecClass, QueueKind, TransferMode};

    #[test]
    fn test_dot_builder_camera_pipeline() {
        // Build AST via the programmatic Dot Builder API
        let ir = WorkflowBuilder::new("CameraPipeline")
            // 1. Define message contracts
            .add_message(MessageBuilder::new("CameraFrame").build())
            // 2. Define interface
            .add_interface(
                InterfaceBuilder::new("CameraTelemetry")
                    .out_port("send_frame", "CameraFrame")
                    .queue(QueueKind::Spsc, 1024)
                    .timeout_ms(2)
                    .done()
                    .in_port("recv_frame", "CameraFrame")
                    .queue(QueueKind::Spsc, 1024)
                    .backpressure(BackpressurePolicy::DropOldest)
                    .done()
                    .build(),
            )
            // 3. Define processes
            .add_process(
                ProcessBuilder::new("CameraIngest", "CameraTelemetry")
                    .exec_class(ExecClass::Thread)
                    .build(),
            )
            .add_process(
                ProcessBuilder::new("FrameProcessor", "CameraTelemetry")
                    .exec_class(ExecClass::Thread)
                    .build(),
            )
            // 4. Define topology routes
            .route(
                "CameraIngest",
                "send_frame",
                "FrameProcessor",
                "recv_frame",
                TransferMode::PublishOffset,
            )
            .build();

        // Validate the resulting IR structure
        assert_eq!(ir.name, "CameraPipeline");
        assert_eq!(ir.messages.len(), 1);
        assert!(ir.messages.contains_key("CameraFrame"));

        assert_eq!(ir.interfaces.len(), 1);
        let iface = ir.interfaces.get("CameraTelemetry").unwrap();
        assert_eq!(iface.ports.len(), 2);
        let out_port = iface.ports.get("send_frame").unwrap();
        assert_eq!(out_port.message_name, "CameraFrame");
        assert_eq!(out_port.timeout_ms, Some(2));
        assert_eq!(out_port.queue_spec.kind, QueueKind::Spsc);

        assert_eq!(ir.processes.len(), 2);
        assert!(ir.processes.contains_key("CameraIngest"));
        assert!(ir.processes.contains_key("FrameProcessor"));

        assert_eq!(ir.routes.len(), 1);
        let route = &ir.routes[0];
        assert_eq!(route.from_process, "CameraIngest");
        assert_eq!(route.to_port, "recv_frame");
        assert_eq!(route.transfer_mode, TransferMode::PublishOffset);
    }
}
