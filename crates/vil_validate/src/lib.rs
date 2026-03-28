// =============================================================================
// vil_validate — Semantic Validation Layer
// =============================================================================
// Validates the VIL Semantic IR against system physics:
// zero-copy constraints, layout legality, queue semantics, boundary rules,
// and ownership tracking.
// =============================================================================

pub mod boundary;
pub mod failover;
pub mod layout;
pub mod memory_class_pass;
pub mod obs_pass;
pub mod ownership;
pub mod queue;
pub mod reactive;
pub mod semantic_lane;
pub mod traits;
pub mod trust_zone;

pub use boundary::BoundaryLegalityPass;
pub use failover::FailoverLegalityPass;
pub use layout::LayoutLegalityPass;
pub use memory_class_pass::MemoryClassCompatibilityPass;
pub use obs_pass::ObsAnnotationPass;
pub use ownership::OwnershipLegalityPass;
pub use queue::QueueCapabilityPass;
pub use reactive::{LaneLegalityPass, ReactiveInterfacePass};
pub use semantic_lane::SemanticLanePass;
pub use trust_zone::TrustZonePass;

pub use traits::{Diagnostic, Severity, ValidationPass, ValidationReport};

use vil_ir::core::WorkflowIR;

/// Main engine for running all validation passes on a Workflow IR.
#[derive(Default)]
pub struct Validator {
    passes: Vec<Box<dyn ValidationPass>>,
}

impl Validator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a standard VIL Validator instance with all passes enabled.
    pub fn standard() -> Self {
        let mut v = Self::new();
        v.add_pass(LayoutLegalityPass);
        v.add_pass(BoundaryLegalityPass);
        v.add_pass(QueueCapabilityPass);
        v.add_pass(OwnershipLegalityPass);
        v.add_pass(LaneLegalityPass);
        v.add_pass(ReactiveInterfacePass);
        v.add_pass(SemanticLanePass);
        v.add_pass(FailoverLegalityPass);
        v.add_pass(TrustZonePass);
        v.add_pass(ObsAnnotationPass);
        v.add_pass(MemoryClassCompatibilityPass);
        v
    }

    pub fn add_pass<P: ValidationPass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    /// Execute validation on the IR and return a compiled report of all issues.
    pub fn validate(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut global_report = ValidationReport::new();

        for pass in &self.passes {
            let pass_report = pass.run(ir);
            global_report.merge(pass_report);
        }

        global_report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_ir::builder::{InterfaceBuilder, MessageBuilder, ProcessBuilder, WorkflowBuilder};
    use vil_types::{CleanupPolicy, LayoutProfile, QueueKind, TransferMode};

    #[test]
    fn test_valid_ir() {
        let ir = WorkflowBuilder::new("ValidFlow")
            .add_message(
                MessageBuilder::new("Msg")
                    .layout(LayoutProfile::Relative)
                    .build(),
            )
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "Msg")
                    .queue(QueueKind::Spsc, 10)
                    .done()
                    .in_port("rx", "Msg")
                    .queue(QueueKind::Spsc, 10)
                    .done()
                    .build(),
            )
            .add_process(
                ProcessBuilder::new("A", "Iface")
                    .cleanup(CleanupPolicy::ReclaimOrphans)
                    .build(),
            )
            .add_process(
                ProcessBuilder::new("B", "Iface")
                    .cleanup(CleanupPolicy::ReclaimOrphans)
                    .build(),
            )
            .route("A", "tx", "B", "rx", TransferMode::LoanWrite)
            .build();

        let validator = Validator::standard();
        let report = validator.validate(&ir);

        // Should have no errors/warnings since the spec is clean and valid.
        assert!(!report.has_errors());
        assert_eq!(report.diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_layout() {
        // External message layout but trying to use zero-copy
        let ir = WorkflowBuilder::new("BadLayoutFlow")
            .add_message(
                MessageBuilder::new("ExternalMsg")
                    .layout(LayoutProfile::External)
                    .build(),
            )
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "ExternalMsg")
                    .queue(QueueKind::Spsc, 10)
                    .done()
                    .in_port("rx", "ExternalMsg")
                    .queue(QueueKind::Spsc, 10)
                    .done()
                    .build(),
            )
            .add_process(ProcessBuilder::new("A", "Iface").build())
            .add_process(ProcessBuilder::new("B", "Iface").build())
            .route("A", "tx", "B", "rx", TransferMode::PublishOffset)
            .build();

        let validator = Validator::standard();
        let report = validator.validate(&ir);

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|d| d.code == "E-LAYOUT-02"));
    }

    #[test]
    fn test_invalid_queue() {
        // 1 producer to 2 consumers using the same tx port as SPSC
        let ir = WorkflowBuilder::new("BadQueueFlow")
            .add_message(
                MessageBuilder::new("Msg")
                    .layout(LayoutProfile::Relative)
                    .build(),
            )
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "Msg")
                    .queue(QueueKind::Spsc, 10)
                    .done()
                    .in_port("rx", "Msg")
                    .queue(QueueKind::Spsc, 10)
                    .done()
                    .build(),
            )
            .add_process(ProcessBuilder::new("A", "Iface").build())
            .add_process(ProcessBuilder::new("B", "Iface").build())
            .add_process(ProcessBuilder::new("C", "Iface").build())
            .route("A", "tx", "B", "rx", TransferMode::LoanWrite)
            .route("A", "tx", "C", "rx", TransferMode::LoanWrite)
            .build();

        let validator = Validator::standard();
        let report = validator.validate(&ir);

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|d| d.code == "E-QUEUE-01"));
    }
}
