// =============================================================================
// vil_validate::boundary — Boundary Legality Pass
// =============================================================================

use vil_ir::core::WorkflowIR;
use vil_types::{BoundaryKind, TransferMode};

use crate::traits::{Diagnostic, ValidationPass, ValidationReport};

pub struct BoundaryLegalityPass;

impl ValidationPass for BoundaryLegalityPass {
    fn name(&self) -> &'static str {
        "BoundaryLegalityPass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        for route in &ir.routes {
            // Rule: InterHost or ForeignRuntime boundaries strongly restrict zero-copy
            let is_zero_copy = matches!(
                route.transfer_mode,
                TransferMode::LoanWrite
                    | TransferMode::LoanRead
                    | TransferMode::PublishOffset
                    | TransferMode::ShareRead
            );

            if is_zero_copy
                && (route.boundary == BoundaryKind::InterHost
                    || route.boundary == BoundaryKind::ForeignRuntime)
            {
                report.push(Diagnostic::error(
                    "E-BOUNDARY-01",
                    format!(
                        "Boundary {:?} prohibits zero-copy transfer {:?}",
                        route.boundary, route.transfer_mode
                    ),
                    format!("Route {} -> {}", route.from_process, route.to_process),
                ));
            }
        }

        report
    }
}
