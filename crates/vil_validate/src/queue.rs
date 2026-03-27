// =============================================================================
// vil_validate::queue — Queue Capability Pass
// =============================================================================

use std::collections::HashMap;
use vil_ir::core::WorkflowIR;
use vil_types::QueueKind;

use crate::traits::{Diagnostic, ValidationPass, ValidationReport};

pub struct QueueCapabilityPass;

impl ValidationPass for QueueCapabilityPass {
    fn name(&self) -> &'static str {
        "QueueCapabilityPass"
    }

    fn run(&self, ir: &WorkflowIR) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Count producers and consumers for each process port
        let mut port_connections: HashMap<(String, String), usize> = HashMap::new();

        for route in &ir.routes {
            let from_key = (route.from_process.clone(), route.from_port.clone());
            let to_key = (route.to_process.clone(), route.to_port.clone());
            
            *port_connections.entry(from_key).or_insert(0) += 1;
            *port_connections.entry(to_key).or_insert(0) += 1;
        }

        // Validate that Spsc queues are only used for 1-to-1 connections
        for ((proc_name, port_name), count) in port_connections {
            if count > 1 {
                // Determine queue type of this port
                if let Some(proc_ir) = ir.processes.get(&proc_name) {
                    if let Some(iface) = ir.interfaces.get(&proc_ir.interface_name) {
                        if let Some(port) = iface.ports.get(&port_name) {
                            if port.queue_spec.kind == QueueKind::Spsc {
                                report.push(Diagnostic::error(
                                    "E-QUEUE-01",
                                    format!("Port '{}' on process '{}' has {} connections but is configured as Spsc. Change to Mpmc.", port_name, proc_name, count),
                                    format!("{}.{}", proc_name, port_name),
                                ));
                            }
                        }
                    }
                }
            }
        }

        report
    }
}
