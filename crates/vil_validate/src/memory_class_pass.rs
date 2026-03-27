// =============================================================================
// vil_validate::memory_class_pass — Memory Class Compatibility
// =============================================================================
// Enforces the transfer-mode compatibility matrix:
//
//   MemoryClass    | LoanWrite | LoanRead | Copy | PublishOffset (RemotePull)
//   ——————————————-+———————————+——————————+——————+——————————————————————————
//   PagedExchange  | ✅        | ✅       | ❌   | ❌
//   PinnedRemote   | ✅        | ✅       | ❌   | ✅
//   ControlHeap    | ❌        | ❌       | ✅   | ❌
//   LocalScratch   | ✅        | ✅       | ✅   | ❌
//
// Raises MCL01 when a route's TransferMode is incompatible with the
// message's declared MemoryClass.
// =============================================================================

use vil_ir::core::WorkflowIR;
use crate::traits::{Diagnostic, ValidationPass, ValidationReport};

pub struct MemoryClassCompatibilityPass;

impl ValidationPass for MemoryClassCompatibilityPass {
    fn name(&self) -> &'static str {
        "MemoryClassCompatibilityPass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        for route in &ir.routes {
            let transfer_mode = route.transfer_mode;

            // Look up the message type from the source port
            // InterfaceIR.ports is a HashMap<String, PortIR>
            let msg_name = ir.interfaces.get(&format!("{}Interface", route.from_process))
                .and_then(|iface| iface.ports.get(&route.from_port))
                .map(|p| p.message_name.clone())
                .or_else(|| {
                    // Try all interfaces to find this port
                    ir.interfaces.values()
                        .find_map(|iface| iface.ports.get(&route.from_port))
                        .map(|p| p.message_name.clone())
                });

            let Some(msg_name) = msg_name else { continue };
            let Some(msg_ir) = ir.messages.get(&msg_name) else { continue };

            let memory_class = msg_ir.memory_class;
            let allowed = memory_class.allowed_transfer_modes();

            if !allowed.contains(&transfer_mode) {
                report.push(Diagnostic::error(
                    "MCL01",
                    format!(
                        "Route '{}.{} -> {}.{}': message '{}' has memory_class '{}' \
                         which does not allow transfer_mode '{}'. \
                         Allowed modes: {}.",
                        route.from_process, route.from_port,
                        route.to_process, route.to_port,
                        msg_name,
                        memory_class,
                        transfer_mode,
                        allowed.iter()
                            .map(|m| format!("{}", m))
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    format!("{}/{}", route.from_process, route.from_port),
                ));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_ir::builder::{WorkflowBuilder, MessageBuilder, InterfaceBuilder, ProcessBuilder};
    use vil_types::{LayoutProfile, QueueKind, TransferMode, MemoryClass, CleanupPolicy};

    fn build_simple_workflow(
        msg_memory_class: MemoryClass,
        route_transfer: TransferMode,
    ) -> WorkflowIR {
        WorkflowBuilder::new("TestFlow")
            .add_message(
                MessageBuilder::new("Payload")
                    .layout(LayoutProfile::Flat)
                    .memory_class(msg_memory_class)
                    .build()
            )
            .add_interface(
                InterfaceBuilder::new("SenderInterface")
                    .out_port("out", "Payload").queue(QueueKind::Spsc, 8).done()
                    .build()
            )
            .add_interface(
                InterfaceBuilder::new("ReceiverInterface")
                    .in_port("in_port", "Payload").queue(QueueKind::Spsc, 8).done()
                    .build()
            )
            .add_process(ProcessBuilder::new("Sender", "SenderInterface")
                .cleanup(CleanupPolicy::ReclaimOrphans).build())
            .add_process(ProcessBuilder::new("Receiver", "ReceiverInterface")
                .cleanup(CleanupPolicy::ReclaimOrphans).build())
            .route_ext("Sender", "out", "Receiver", "in_port", route_transfer, None)
            .build()
    }

    #[test]
    fn test_paged_exchange_loan_write_ok() {
        let ir = build_simple_workflow(MemoryClass::PagedExchange, TransferMode::LoanWrite);
        let report = MemoryClassCompatibilityPass.run(&ir);
        assert!(!report.has_errors(),
            "PagedExchange + LoanWrite should be valid: {:?}", report.diagnostics);
    }

    #[test]
    fn test_control_heap_loan_write_illegal() {
        let ir = build_simple_workflow(MemoryClass::ControlHeap, TransferMode::LoanWrite);
        let report = MemoryClassCompatibilityPass.run(&ir);
        assert!(report.has_errors(), "ControlHeap + LoanWrite should raise MCL01");
        assert!(report.diagnostics.iter().any(|d| d.code == "MCL01"),
            "Expected MCL01 diagnostic, got: {:?}", report.diagnostics);
    }

    #[test]
    fn test_control_heap_copy_ok() {
        let ir = build_simple_workflow(MemoryClass::ControlHeap, TransferMode::Copy);
        let report = MemoryClassCompatibilityPass.run(&ir);
        assert!(!report.has_errors(),
            "ControlHeap + Copy should be valid: {:?}", report.diagnostics);
    }

    #[test]
    fn test_pinned_remote_copy_illegal() {
        let ir = build_simple_workflow(MemoryClass::PinnedRemote, TransferMode::Copy);
        let report = MemoryClassCompatibilityPass.run(&ir);
        assert!(report.has_errors(), "PinnedRemote + Copy should raise MCL01");
        assert!(report.diagnostics.iter().any(|d| d.code == "MCL01"));
    }

    #[test]
    fn test_local_scratch_copy_ok() {
        let ir = build_simple_workflow(MemoryClass::LocalScratch, TransferMode::Copy);
        let report = MemoryClassCompatibilityPass.run(&ir);
        assert!(!report.has_errors(),
            "LocalScratch + Copy should be valid: {:?}", report.diagnostics);
    }
}
