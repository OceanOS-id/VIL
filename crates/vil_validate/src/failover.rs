// =============================================================================
// crates/vil_validate/src/failover.rs
// =============================================================================

use crate::traits::{Diagnostic, ValidationPass, ValidationReport};
use vil_ir::core::WorkflowIR;

/// Validation rule to ensure Failover strategies are legally defined.
/// Checks that:
/// 1. The source process exists.
/// 2. If the target is a process (not a retry strategy), the target process exists.
/// 3. The target process (if applicable) must have at least all the input/output ports of the source process, ensuring interface compatibility.
pub struct FailoverLegalityPass;

impl ValidationPass for FailoverLegalityPass {
    fn name(&self) -> &'static str {
        "FailoverLegalityPass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        for failover in &ir.failovers {
            // Source must exist
            let source_proc = match ir.processes.get(&failover.source) {
                Some(p) => p,
                None => {
                    report.push(Diagnostic::error(
                        "FA01",
                        format!("Failover source process '{}' does not exist in workflow.", failover.source),
                        failover.source.clone(),
                    ));
                    continue;
                }
            };

            // If target starts with "retry(", it is a retry strategy, skip process checks
            if failover.target.starts_with("retry(") {
                continue;
            }

            // Target must exist
            let target_proc = match ir.processes.get(&failover.target) {
                Some(p) => p,
                None => {
                    report.push(Diagnostic::error(
                        "FA02",
                        format!("Failover target process '{}' does not exist in workflow.", failover.target),
                        failover.target.clone(),
                    ));
                    continue;
                }
            };

            // Target must be compatible (must have all ports from source)
            let src_iface = match ir.interfaces.get(&source_proc.interface_name) {
                Some(i) => i,
                None => continue, // Should be caught by another pass
            };

            let mut target_has_compatible_iface = false;

            if let Some(target_iface) = ir.interfaces.get(&target_proc.interface_name) {
                // Check if the target has all the ports
                let mut all_ports_found = true;
                for (_, src_port) in &src_iface.ports {
                    let has_port = target_iface.ports.iter().any(|(_, target_port)| {
                        target_port.name == src_port.name
                            && target_port.direction == src_port.direction
                            && target_port.message_name == src_port.message_name
                    });
                    
                    if !has_port {
                        all_ports_found = false;
                        break;
                    }
                }

                if all_ports_found {
                    target_has_compatible_iface = true;
                }
            }

            if !target_has_compatible_iface {
                report.push(Diagnostic::error(
                    "FA03",
                    format!(
                        "Failover target '{}' is incompatible. It lacks the ports defined in source '{}'s interface '{}'.",
                        failover.target, failover.source, src_iface.name
                    ),
                    failover.target.clone(),
                ));
            }
        }

        report
    }
}
