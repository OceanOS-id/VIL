// =============================================================================
// vil_validate::ownership — Ownership Legality Pass
// =============================================================================

use vil_ir::core::WorkflowIR;
use vil_types::{CleanupPolicy, TransferMode};

use crate::traits::{Diagnostic, ValidationPass, ValidationReport};

pub struct OwnershipLegalityPass;

impl ValidationPass for OwnershipLegalityPass {
    fn name(&self) -> &'static str {
        "OwnershipLegalityPass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        for route in &ir.routes {
            // Rule: LoanWrite and LoanRead mandate ReclaimOrphans if crash occurs
            let implies_ownership_risk = matches!(
                route.transfer_mode,
                TransferMode::LoanWrite | TransferMode::LoanRead | TransferMode::PublishOffset
            );

            if implies_ownership_risk {
                let mut check_proc = |proc_name: &str, role: &str| {
                    if let Some(proc_ir) = ir.processes.get(proc_name) {
                        if proc_ir.cleanup_policy != CleanupPolicy::ReclaimOrphans {
                            report.push(Diagnostic::warning(
                                "W-OWNERSHIP-01",
                                format!("Process '{}' acts as {} using zero-copy transfer {:?} but its cleanup policy is {:?}", proc_name, role, route.transfer_mode, proc_ir.cleanup_policy),
                                proc_name.to_string(),
                            ));
                        }
                    }
                };
                
                check_proc(&route.from_process, "producer");
                check_proc(&route.to_process, "consumer");
            }
        }

        // Ownership leak detection
        for transfer in &ir.transfers {
            let implies_ownership = matches!(
                transfer.transfer_mode,
                TransferMode::LoanWrite | TransferMode::LoanRead | TransferMode::PublishOffset
            );

            if implies_ownership {
                let has_published = transfer.expected_flow.contains(&vil_ir::OwnershipState::Published);
                let has_released = transfer.expected_flow.contains(&vil_ir::OwnershipState::Released);

                if has_published && !has_released {
                    report.push(Diagnostic::error(
                        "E-OWNERSHIP-LEAK-01",
                        format!(
                            "Transfer '{}' (mode {:?}) expects 'Published' state but missing 'Released' state. This will cause a memory leak.",
                            transfer.name, transfer.transfer_mode
                        ),
                        &transfer.name,
                    ));
                }
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_ir::builder::*;
    use vil_types::{QueueKind, CleanupPolicy};
    use vil_ir::{OwnershipState, TransferExprIR};

    #[test]
    fn test_ownership_leak_detection() {
        let mut ir = WorkflowBuilder::new("LeakDemo")
            .add_message(MessageBuilder::new("Data").build())
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "Data").queue(QueueKind::Spsc, 10).done()
                    .in_port("rx", "Data").queue(QueueKind::Spsc, 10).done()
                    .build()
            )
            .add_process(ProcessBuilder::new("Producer", "Iface").cleanup(CleanupPolicy::ReclaimOrphans).build())
            .add_process(ProcessBuilder::new("Consumer", "Iface").cleanup(CleanupPolicy::ReclaimOrphans).build())
            .route("Producer", "tx", "Consumer", "rx", TransferMode::LoanWrite)
            .build();

        // 1. Manually add a valid transfer
        ir.transfers.push(TransferExprIR {
            name: "valid_transfer".into(),
            from_process: "Producer".into(),
            from_port: "tx".into(),
            to_process: "Consumer".into(),
            to_port: "rx".into(),
            transfer_mode: TransferMode::LoanWrite,
            message_name: "Data".into(),
            expected_flow: vec![OwnershipState::Allocated, OwnershipState::Published, OwnershipState::Received, OwnershipState::Released],
        });

        let pass = OwnershipLegalityPass;
        let report = pass.run(&ir);
        assert!(!report.has_errors(), "Valid transfer should not have errors");

        // 2. Add an invalid transfer (leak)
        ir.transfers.clear();
        ir.transfers.push(TransferExprIR {
            name: "leaking_transfer".into(),
            from_process: "Producer".into(),
            from_port: "tx".into(),
            to_process: "Consumer".into(),
            to_port: "rx".into(),
            transfer_mode: TransferMode::LoanWrite,
            message_name: "Data".into(),
            // Missing Released
            expected_flow: vec![OwnershipState::Allocated, OwnershipState::Published, OwnershipState::Received],
        });

        let report = pass.run(&ir);
        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|d| d.code == "E-OWNERSHIP-LEAK-01"));
    }
}
