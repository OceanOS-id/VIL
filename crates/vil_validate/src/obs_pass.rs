// =============================================================================
// vil_validate::obs_pass — Observability Annotation Pass
// =============================================================================
// Validates #[trace_hop] and #[latency_marker] annotations on processes.
// Rules:
//   OBS01: latency_label must be non-empty string when set
//   OBS02: trace_hop=true process with no routes is a no-op (warning)
// =============================================================================

use vil_ir::core::WorkflowIR;
use crate::traits::{Diagnostic, ValidationPass, ValidationReport};

pub struct ObsAnnotationPass;

impl ValidationPass for ObsAnnotationPass {
    fn name(&self) -> &'static str {
        "ObsAnnotationPass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Collect all process names that appear in at least one route
        let routed: std::collections::HashSet<&str> = ir.routes.iter()
            .flat_map(|r| [r.from_process.as_str(), r.to_process.as_str()])
            .collect();

        for (name, process) in &ir.processes {
            let obs = &process.obs;

            // OBS01: latency_label must be non-empty when set
            if let Some(ref label) = obs.latency_label {
                if label.trim().is_empty() {
                    report.push(Diagnostic::error(
                        "OBS01",
                        format!(
                            "Process '{}' has #[latency_marker] with an empty label. \
                             Provide a non-empty label for dashboarding.",
                            name
                        ),
                        name,
                    ));
                }
            }

            // OBS02: trace_hop on an isolated process is a no-op — warn
            if obs.trace_hop && !routed.contains(name.as_str()) {
                report.push(Diagnostic::warning(
                    "OBS02",
                    format!(
                        "Process '{}' has #[trace_hop] but appears in no routes. \
                         Hop tracing will never fire.",
                        name
                    ),
                    name,
                ));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_ir::core::{WorkflowIR, ProcessIR, ObsIR};
    use vil_types::{ExecClass, CleanupPolicy, Priority};

    fn make_process(name: &str, obs: ObsIR) -> ProcessIR {
        ProcessIR {
            name: name.to_string(),
            interface_name: format!("{}Iface", name),
            exec_class: ExecClass::Thread,
            cleanup_policy: CleanupPolicy::ReclaimOrphans,
            priority: Priority::Normal,
            host_affinity: None,
            trust_zone: None,
            obs,
        }
    }

    fn add_route(ir: &mut WorkflowIR, from: &str, to: &str) {
        ir.routes.push(vil_ir::core::RouteIR {
            from_process: from.to_string(),
            from_port: "out".to_string(),
            to_process: to.to_string(),
            to_port: "in_port".to_string(),
            transfer_mode: vil_types::TransferMode::Copy,
            boundary: vil_types::BoundaryKind::InterThreadLocal,
            scope: vil_ir::core::RouteScope::Local,
            transport: None,
        });
    }

    #[test]
    fn test_valid_annotations() {
        let mut ir = WorkflowIR::new("TestWf");
        ir.processes.insert("Inference".into(), make_process("Inference", ObsIR {
            trace_hop: true,
            latency_label: Some("inference".into()),
        }));
        ir.processes.insert("Next".into(), make_process("Next", ObsIR::default()));
        add_route(&mut ir, "Inference", "Next");

        let report = ObsAnnotationPass.run(&ir);
        assert!(!report.has_errors(), "Should have no errors: {:?}", report.diagnostics);
    }

    #[test]
    fn test_empty_label_error() {
        let mut ir = WorkflowIR::new("TestWf");
        ir.processes.insert("Node".into(), make_process("Node", ObsIR {
            trace_hop: false,
            latency_label: Some("".into()), // empty!
        }));

        let report = ObsAnnotationPass.run(&ir);
        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|d| d.code == "OBS01"));
    }

    #[test]
    fn test_isolated_trace_hop_warning() {
        let mut ir = WorkflowIR::new("TestWf");
        ir.processes.insert("Orphan".into(), make_process("Orphan", ObsIR {
            trace_hop: true,
            latency_label: None,
        }));
        // No routes — Orphan is isolated

        let report = ObsAnnotationPass.run(&ir);
        assert!(!report.has_errors(), "Should be a warning, not an error");
        assert!(report.diagnostics.iter().any(|d| d.code == "OBS02"));
    }
}
