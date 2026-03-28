// =============================================================================
// crates/vil_validate/src/trust_zone.rs — TrustZone Enforcement
// =============================================================================
// Verifies that data flow between processes does not violate the
// capability constraints of each process's trust zone.
// =============================================================================

use crate::traits::{Diagnostic, ValidationPass, ValidationReport};
use vil_ir::core::WorkflowIR;
use vil_types::{zone_capabilities, TrustZone, ZoneCapability};

/// Validation pass that enforces capability constraints across Trust Zones.
pub struct TrustZonePass;

/// Determine if a trust zone is allowed to emit in data lanes (connect via routes).
fn can_emit(zone: TrustZone) -> bool {
    zone_capabilities(zone).contains(&ZoneCapability::CanEmitLane)
}

/// Determine if a cross-zone route is allowed.
/// Core rule: ExternalBoundary can receive data but must not emit.
fn is_route_allowed(src_zone: TrustZone, _dst_zone: TrustZone) -> bool {
    can_emit(src_zone)
}

impl ValidationPass for TrustZonePass {
    fn name(&self) -> &'static str {
        "TrustZonePass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        for route in &ir.routes {
            let src_zone = ir
                .processes
                .get(&route.from_process)
                .and_then(|p| p.trust_zone);

            let dst_zone = ir
                .processes
                .get(&route.to_process)
                .and_then(|p| p.trust_zone);

            // Only check cross-zone routes where both zones are known
            match (src_zone, dst_zone) {
                (Some(src), Some(dst)) if src != dst => {
                    if !is_route_allowed(src, dst) {
                        report.push(Diagnostic::error(
                            "TZ01",
                            format!(
                                "Route '{}.{} -> {}.{}' violates trust boundary: '{}' cannot emit to '{}'.",
                                route.from_process, route.from_port,
                                route.to_process, route.to_port,
                                src, dst,
                            ),
                            route.from_process.clone(),
                        ));
                    }
                }
                (Some(src), None) => {
                    // Source has zone but destination doesn't — flag as unknown
                    report.push(Diagnostic::error(
                        "TZ02",
                        format!(
                            "Process '{}' has trust_zone = {} but its route destination '{}' has no trust_zone declared.",
                            route.from_process, src, route.to_process,
                        ),
                        route.to_process.clone(),
                    ));
                }
                _ => {} // Same zone or both zones unknown — pass
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_types::{BoundaryKind, CleanupPolicy, ExecClass, Priority, TransferMode};

    fn make_process(name: &str, zone: TrustZone) -> vil_ir::core::ProcessIR {
        vil_ir::core::ProcessIR {
            name: name.to_string(),
            interface_name: format!("{}Iface", name),
            exec_class: ExecClass::Thread,
            cleanup_policy: CleanupPolicy::ReclaimOrphans,
            priority: Priority::Normal,
            host_affinity: None,
            trust_zone: Some(zone),
            obs: vil_ir::core::ObsIR::default(),
        }
    }

    #[test]
    fn test_external_cannot_emit_to_trusted() {
        let mut ir = vil_ir::core::WorkflowIR::new("TestWorkflow");
        ir.processes.insert(
            "Adapter".to_string(),
            make_process("Adapter", TrustZone::ExternalBoundary),
        );
        ir.processes.insert(
            "CoreProc".to_string(),
            make_process("CoreProc", TrustZone::NativeTrusted),
        );
        ir.routes.push(vil_ir::core::RouteIR {
            from_process: "Adapter".to_string(),
            from_port: "out".to_string(),
            to_process: "CoreProc".to_string(),
            to_port: "in_port".to_string(),
            transfer_mode: TransferMode::Copy,
            boundary: BoundaryKind::InterThreadLocal,
            scope: vil_ir::core::RouteScope::Local,
            transport: None,
        });

        let pass = TrustZonePass;
        let report = pass.run(&ir);
        assert!(report.has_errors(), "Should have reported TZ01 violation");
    }

    #[test]
    fn test_trusted_can_emit_to_wasm() {
        let mut ir = vil_ir::core::WorkflowIR::new("TestWorkflow");
        ir.processes.insert(
            "Trusted".to_string(),
            make_process("Trusted", TrustZone::NativeTrusted),
        );
        ir.processes.insert(
            "Plugin".to_string(),
            make_process("Plugin", TrustZone::WasmCapsule),
        );
        ir.routes.push(vil_ir::core::RouteIR {
            from_process: "Trusted".to_string(),
            from_port: "output".to_string(),
            to_process: "Plugin".to_string(),
            to_port: "input".to_string(),
            transfer_mode: TransferMode::Copy,
            boundary: BoundaryKind::InterThreadLocal,
            scope: vil_ir::core::RouteScope::Local,
            transport: None,
        });

        let pass = TrustZonePass;
        let report = pass.run(&ir);
        assert!(
            !report.has_errors(),
            "NativeTrusted should be allowed to emit to WasmCapsule"
        );
    }
}
