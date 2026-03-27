// =============================================================================
// vil_validate::semantic_lane — Lane-Aware Payload Classifier
// =============================================================================
// Validates that semantic types are only sent to their permitted lanes:
//   - State    -> Data Lane only
//   - Event    -> Data Lane or Control Lane
//   - Fault    -> Control Lane only
//   - Decision -> Trigger Lane only
//   - Message  -> any lane (backward-compatible)
//
// Also validates transfer mode compatibility per semantic kind.
//
// Produces compile-time errors if a type is sent to an incorrect lane.
// =============================================================================

use vil_ir::core::WorkflowIR;
use vil_types::LaneKind;

use crate::traits::{Diagnostic, ValidationPass, ValidationReport};

/// Lane-Aware Payload Classifier — validates that semantic types
/// only flow through their permitted lanes.
///
/// Rules:
/// 1. Each route sending a message to a port with a specific lane
///    must be compatible with `SemanticKind::allowed_lanes()`.
/// 2. The route's transfer mode must be compatible with
///    `SemanticKind::allowed_transfer_modes()`.
/// 3. Types with `MemoryClass::ControlHeap` must not be sent via LoanWrite.
pub struct SemanticLanePass;

impl ValidationPass for SemanticLanePass {
    fn name(&self) -> &'static str {
        "SemanticLanePass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        // For each route, check whether the sent message type
        // is compatible with the destination port's lane.
        for route in &ir.routes {
            // Find the destination port in the interface
            let dest_port = ir.interfaces.values()
                .flat_map(|iface| iface.ports.values())
                .find(|p| p.name == route.to_port);

            let dest_port = match dest_port {
                Some(p) => p,
                None => continue, // Port not found — skip (handled by other passes)
            };

            // Also find the source port to determine message name
            let src_port = ir.interfaces.values()
                .flat_map(|iface| iface.ports.values())
                .find(|p| p.name == route.from_port);

            let src_port = match src_port {
                Some(p) => p,
                None => continue,
            };

            // Skip if the port uses Default lane (backward-compatible)
            if dest_port.lane_kind == LaneKind::Default {
                continue;
            }

            // Look up MessageIR by name from the source port
            let message = ir.messages.get(&src_port.message_name);
            let message = match message {
                Some(m) => m,
                None => continue, // Message type not yet defined — skip
            };

            let context = format!(
                "route {}.{} -> {}.{} (message '{}', kind={})",
                route.from_process, route.from_port,
                route.to_process, route.to_port,
                message.name, message.semantic_kind
            );

            // --- Rule 1: Lane compatibility ---
            let allowed_lanes = message.semantic_kind.allowed_lanes();
            if !allowed_lanes.contains(&dest_port.lane_kind) {
                report.push(Diagnostic::error(
                    "E-SEMANTIC-LANE-01",
                    format!(
                        "Semantic type '{}' (kind={}) is not allowed on {} lane. Allowed lanes: {:?}",
                        message.name,
                        message.semantic_kind,
                        dest_port.lane_kind,
                        allowed_lanes
                    ),
                    &context,
                ));
            }

            // --- Rule 2: Transfer mode compatibility ---
            let allowed_modes = message.semantic_kind.allowed_transfer_modes();
            if !allowed_modes.contains(&route.transfer_mode) {
                report.push(Diagnostic::error(
                    "E-SEMANTIC-LANE-02",
                    format!(
                        "Transfer mode '{}' is not allowed for semantic kind '{}'. Allowed: {:?}",
                        route.transfer_mode,
                        message.semantic_kind,
                        allowed_modes
                    ),
                    &context,
                ));
            }

            // --- Rule 3: ControlHeap messages must not use LoanWrite ---
            if message.memory_class == vil_types::MemoryClass::ControlHeap
                && route.transfer_mode == vil_types::TransferMode::LoanWrite
            {
                report.push(Diagnostic::error(
                    "E-SEMANTIC-LANE-03",
                    format!(
                        "ControlHeap message '{}' cannot be sent via LoanWrite. Use Copy instead.",
                        message.name
                    ),
                    &context,
                ));
            }

            // --- Rule 4: Large payload warning on Trigger Lane ---
            if dest_port.lane_kind == LaneKind::Trigger {
                let estimated_size = estimate_message_size(message);
                if estimated_size > 64 {
                    report.push(Diagnostic::warning(
                        "W-SEMANTIC-LANE-01",
                        format!(
                            "Message '{}' (~{} bytes) sent to Trigger Lane exceeds 64-byte soft limit. Consider using Data Lane for large payloads.",
                            message.name,
                            estimated_size
                        ),
                        &context,
                    ));
                }
            }
        }

        report
    }
}

/// Estimates message size based on field types.
/// Simple heuristic — not an actual sizeof().
fn estimate_message_size(msg: &vil_ir::core::MessageIR) -> usize {
    use vil_ir::core::TypeRefIR;
    let mut total = 0;
    for field in &msg.fields {
        total += match &field.ty {
            TypeRefIR::Primitive(name) => match name.as_str() {
                "u8" | "i8" | "bool" => 1,
                "u16" | "i16" => 2,
                "u32" | "i32" | "f32" => 4,
                "u64" | "i64" | "f64" => 8,
                "u128" | "i128" => 16,
                _ => 8, // default
            },
            TypeRefIR::VSlice(_) => 16, // offset + len
            TypeRefIR::VRef(_) => 8,    // relative offset
            TypeRefIR::Named(_) => 8,   // assume small struct ref
            TypeRefIR::Unknown(_) => 8, // conservative default
        };
    }
    if total == 0 { 8 } else { total } // minimum 8 bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_ir::builder::*;
    use vil_types::*;

    #[test]
    fn test_fault_on_data_lane_rejected() {
        // Fault should only be on Control Lane, not Data Lane
        let ir = WorkflowBuilder::new("FaultOnDataLane")
            .add_message(
                MessageBuilder::new("MyFault")
                    .semantic_kind(SemanticKind::Fault)
                    .memory_class(MemoryClass::ControlHeap)
                    .build()
            )
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "MyFault").lane(LaneKind::Data).queue(QueueKind::Spsc, 10).done()
                    .in_port("rx", "MyFault").lane(LaneKind::Data).queue(QueueKind::Spsc, 10).done()
                    .build()
            )
            .add_process(ProcessBuilder::new("A", "Iface").build())
            .add_process(ProcessBuilder::new("B", "Iface").build())
            .route("A", "tx", "B", "rx", TransferMode::Copy)
            .build();

        let pass = SemanticLanePass;
        let report = pass.run(&ir);

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|d| d.code == "E-SEMANTIC-LANE-01"));
    }

    #[test]
    fn test_fault_on_control_lane_accepted() {
        // Fault on Control Lane is fine
        let ir = WorkflowBuilder::new("FaultOnControlLane")
            .add_message(
                MessageBuilder::new("MyFault")
                    .semantic_kind(SemanticKind::Fault)
                    .memory_class(MemoryClass::ControlHeap)
                    .build()
            )
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "MyFault").lane(LaneKind::Control).queue(QueueKind::Spsc, 10).done()
                    .in_port("rx", "MyFault").lane(LaneKind::Control).queue(QueueKind::Spsc, 10).done()
                    .build()
            )
            .add_process(ProcessBuilder::new("A", "Iface").build())
            .add_process(ProcessBuilder::new("B", "Iface").build())
            .route("A", "tx", "B", "rx", TransferMode::Copy)
            .build();

        let pass = SemanticLanePass;
        let report = pass.run(&ir);

        assert!(!report.has_errors());
    }

    #[test]
    fn test_decision_on_trigger_lane_accepted() {
        let ir = WorkflowBuilder::new("DecisionOnTrigger")
            .add_message(
                MessageBuilder::new("MyDecision")
                    .semantic_kind(SemanticKind::Decision)
                    .memory_class(MemoryClass::ControlHeap)
                    .build()
            )
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "MyDecision").lane(LaneKind::Trigger).queue(QueueKind::Mpmc, 10).done()
                    .in_port("rx", "MyDecision").lane(LaneKind::Trigger).queue(QueueKind::Mpmc, 10).done()
                    .build()
            )
            .add_process(ProcessBuilder::new("A", "Iface").build())
            .add_process(ProcessBuilder::new("B", "Iface").build())
            .route("A", "tx", "B", "rx", TransferMode::Copy)
            .build();

        let pass = SemanticLanePass;
        let report = pass.run(&ir);

        assert!(!report.has_errors());
    }

    #[test]
    fn test_decision_on_data_lane_rejected() {
        let ir = WorkflowBuilder::new("DecisionOnDataLane")
            .add_message(
                MessageBuilder::new("MyDecision")
                    .semantic_kind(SemanticKind::Decision)
                    .memory_class(MemoryClass::ControlHeap)
                    .build()
            )
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "MyDecision").lane(LaneKind::Data).queue(QueueKind::Spsc, 10).done()
                    .in_port("rx", "MyDecision").lane(LaneKind::Data).queue(QueueKind::Spsc, 10).done()
                    .build()
            )
            .add_process(ProcessBuilder::new("A", "Iface").build())
            .add_process(ProcessBuilder::new("B", "Iface").build())
            .route("A", "tx", "B", "rx", TransferMode::LoanWrite)
            .build();

        let pass = SemanticLanePass;
        let report = pass.run(&ir);

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|d| d.code == "E-SEMANTIC-LANE-01"));
    }

    #[test]
    fn test_state_on_data_lane_accepted() {
        let ir = WorkflowBuilder::new("StateOnDataLane")
            .add_message(
                MessageBuilder::new("MyState")
                    .semantic_kind(SemanticKind::State)
                    .build()
            )
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "MyState").lane(LaneKind::Data).queue(QueueKind::Spsc, 10).done()
                    .in_port("rx", "MyState").lane(LaneKind::Data).queue(QueueKind::Spsc, 10).done()
                    .build()
            )
            .add_process(ProcessBuilder::new("A", "Iface").build())
            .add_process(ProcessBuilder::new("B", "Iface").build())
            .route("A", "tx", "B", "rx", TransferMode::LoanWrite)
            .build();

        let pass = SemanticLanePass;
        let report = pass.run(&ir);

        assert!(!report.has_errors());
    }

    #[test]
    fn test_control_heap_via_loan_write_rejected() {
        // ControlHeap messages must use Copy, not LoanWrite
        let ir = WorkflowBuilder::new("ControlHeapLoanWrite")
            .add_message(
                MessageBuilder::new("MyFault")
                    .semantic_kind(SemanticKind::Fault)
                    .memory_class(MemoryClass::ControlHeap)
                    .build()
            )
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "MyFault").lane(LaneKind::Control).queue(QueueKind::Spsc, 10).done()
                    .in_port("rx", "MyFault").lane(LaneKind::Control).queue(QueueKind::Spsc, 10).done()
                    .build()
            )
            .add_process(ProcessBuilder::new("A", "Iface").build())
            .add_process(ProcessBuilder::new("B", "Iface").build())
            .route("A", "tx", "B", "rx", TransferMode::LoanWrite)
            .build();

        let pass = SemanticLanePass;
        let report = pass.run(&ir);

        assert!(report.has_errors());
        // Should have both E-SEMANTIC-LANE-02 (wrong transfer mode for Fault)
        // and E-SEMANTIC-LANE-03 (ControlHeap via LoanWrite)
        assert!(report.diagnostics.iter().any(|d| d.code == "E-SEMANTIC-LANE-02"));
        assert!(report.diagnostics.iter().any(|d| d.code == "E-SEMANTIC-LANE-03"));
    }

    #[test]
    fn test_message_on_any_lane_accepted() {
        // Generic Message (backward-compatible) should work on any lane
        for lane in &[LaneKind::Trigger, LaneKind::Data, LaneKind::Control] {
            let ir = WorkflowBuilder::new("MessageAnyLane")
                .add_message(
                    MessageBuilder::new("GenericMsg")
                        .semantic_kind(SemanticKind::Message)
                        .build()
                )
                .add_interface(
                    InterfaceBuilder::new("Iface")
                        .out_port("tx", "GenericMsg").lane(*lane).queue(QueueKind::Spsc, 10).done()
                        .in_port("rx", "GenericMsg").lane(*lane).queue(QueueKind::Spsc, 10).done()
                        .build()
                )
                .add_process(ProcessBuilder::new("A", "Iface").build())
                .add_process(ProcessBuilder::new("B", "Iface").build())
                .route("A", "tx", "B", "rx", TransferMode::LoanWrite)
                .build();

            let pass = SemanticLanePass;
            let report = pass.run(&ir);

            assert!(!report.has_errors(), "Message should be allowed on {:?} lane", lane);
        }
    }
}
