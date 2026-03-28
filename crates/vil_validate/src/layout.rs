// =============================================================================
// vil_validate::layout — Layout Legality Pass
// =============================================================================

use vil_ir::core::WorkflowIR;
use vil_types::{LayoutProfile, TransferMode};

use crate::traits::{Diagnostic, ValidationPass, ValidationReport};

pub struct LayoutLegalityPass;

impl ValidationPass for LayoutLegalityPass {
    fn name(&self) -> &'static str {
        "LayoutLegalityPass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        // 1. Check existing routes for zero-copy legality (Original check)
        for route in &ir.routes {
            let iface_name = match ir.processes.get(&route.from_process) {
                Some(p) => &p.interface_name,
                None => {
                    report.push(Diagnostic::error(
                        "E-LAYOUT-01",
                        format!("Process not found: {}", route.from_process),
                        "Route Validation",
                    ));
                    continue;
                }
            };

            let port = match ir
                .interfaces
                .get(iface_name)
                .and_then(|i| i.ports.get(&route.from_port))
            {
                Some(p) => p,
                None => continue,
            };

            let msg = match ir.messages.get(&port.message_name) {
                Some(m) => m,
                None => continue,
            };

            let is_zero_copy_transfer = matches!(
                route.transfer_mode,
                TransferMode::LoanWrite
                    | TransferMode::LoanRead
                    | TransferMode::PublishOffset
                    | TransferMode::ShareRead
            );

            if msg.layout == LayoutProfile::External && is_zero_copy_transfer {
                report.push(Diagnostic::error(
                    "E-LAYOUT-02",
                    format!(
                        "Message '{}' has External layout, but route uses zero-copy transfer {:?}",
                        msg.name, route.transfer_mode
                    ),
                    format!("Route {} -> {}", route.from_process, route.to_process),
                ));
            }
        }

        // 2. Deep Field Validation (Phase 1 Goal)
        for msg in ir.messages.values() {
            if msg.layout == LayoutProfile::External {
                continue; // External layout allows anything
            }

            for field in &msg.fields {
                if !is_type_vasi_compliant(&field.ty) {
                    report.push(Diagnostic::error(
                        "E-LAYOUT-03",
                        format!(
                            "Message '{}' field '{}' has non-VASI type '{:?}'. Only POD or VRef/VSlice allowed in shared memory.",
                            msg.name, field.name, field.ty
                        ),
                        format!("Message Definition: {}", msg.name),
                    ));
                }
            }
        }

        report
    }
}

pub fn is_type_vasi_compliant(ty: &vil_ir::core::TypeRefIR) -> bool {
    use vil_ir::core::TypeRefIR;
    match ty {
        TypeRefIR::Primitive(_) => true,
        TypeRefIR::VRef(_) => true,
        TypeRefIR::VSlice(_) => true,
        TypeRefIR::Named(_) => true, // We assume other named VIL messages are also checked
        TypeRefIR::Unknown(name) => {
            // Dangerous types known to be non-VASI
            let forbidden = ["String", "Vec", "Box", "HashMap", "Arc", "Rc"];
            !forbidden.iter().any(|&f| name.contains(f))
        }
    }
}
