// =============================================================================
// vil_validate::reactive — Reactive Primitives Validation
// =============================================================================
// Validation passes for LaneKind and ReactiveInterfaceKind usage,
// enforcing the Tri-Lane Reactive Pattern constraints.
// =============================================================================

use vil_ir::core::WorkflowIR;
use vil_types::{LaneKind, QueueKind, ReactiveInterfaceKind};

use crate::traits::{Diagnostic, ValidationPass, ValidationReport};

// -----------------------------------------------------------------------------
// 1. LaneLegalityPass
// -----------------------------------------------------------------------------

/// Validates port configuration based on lane semantics (`LaneKind`).
///
/// Rules:
/// 1. Trigger lanes (`LaneKind::Trigger`) typically receive data from
///    external ecosystems (HTTP servers, P2P threadpools). Their backing
///    queue (handoff) is safer as MPMC.
/// 2. Control lanes (`LaneKind::Control`) should be filtered from heavy
///    data and focused on `ControlSignal` (out-of-band messages).
pub struct LaneLegalityPass;

impl ValidationPass for LaneLegalityPass {
    fn name(&self) -> &'static str {
        "LaneLegalityPass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        for (iface_name, iface) in &ir.interfaces {
            for (port_name, port) in &iface.ports {
                let context = format!("interface '{}', port '{}'", iface_name, port_name);

                // Rule 1: Trigger handoff should be MPMC
                if port.lane_kind == LaneKind::Trigger && port.queue_spec.kind == QueueKind::Spsc {
                    report.push(Diagnostic::warning(
                        "W-LANE-01",
                        "Trigger lane using SPSC queue. External handoffs (e.g. from web servers) often involve multiple threads. MPMC is safer to avoid congestion.",
                        &context,
                    ));
                }

                // Rule 2: Control lane message type
                // (Control messages should preferably be GenericToken or ControlSignal)
                if port.lane_kind == LaneKind::Control {
                    if port.message_name != "ControlSignal" && port.message_name != "GenericToken" {
                        report.push(Diagnostic::warning(
                            "W-LANE-02",
                            format!(
                                "Control lane uses heavy payload '{}'. Consider using 'ControlSignal' for out-of-band signals.",
                                port.message_name
                            ),
                            &context,
                        ));
                    }
                }
            }
        }

        report
    }
}

// -----------------------------------------------------------------------------
// 2. ReactiveInterfacePass
// -----------------------------------------------------------------------------

/// Validates interface structure based on reactive classification (`ReactiveInterfaceKind`).
///
/// Rules:
/// 1. If an interface is `SessionReactive`, it must have at least one port
///    with `LaneKind::Control` so that session lifecycle signals
///    (DONE/ERROR/ABORT) are not blocked by payload data.
pub struct ReactiveInterfacePass;

impl ValidationPass for ReactiveInterfacePass {
    fn name(&self) -> &'static str {
        "ReactiveInterfacePass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        for (iface_name, iface) in &ir.interfaces {
            let context = format!("interface '{}'", iface_name);

            if iface.reactive_kind == ReactiveInterfaceKind::SessionReactive {
                // Ensure a Control Lane exists
                let has_control = iface.ports.values().any(|p| p.lane_kind == LaneKind::Control);
                
                if !has_control {
                    report.push(Diagnostic::error(
                        "E-REACTIVE-01",
                        "SessionReactive interface must have at least one port with LaneKind::Control to ensure deterministic session termination.",
                        &context,
                    ));
                }

                // Ensure a Data Lane exists (Trigger or Data)
                let has_data = iface.ports.values().any(|p| p.lane_kind == LaneKind::Data || p.lane_kind == LaneKind::Trigger);
                if !has_data {
                    report.push(Diagnostic::error(
                        "E-REACTIVE-02",
                        "SessionReactive interface missing Data/Trigger lane. A session fabric requires data payload delivery.",
                        &context,
                    ));
                }
            }
        }

        report
    }
}
