// =============================================================================
// process.rs — Code Generator for ProcessIR & PortSpecs
// =============================================================================

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use vil_ir::core::WorkflowIR;
use vil_types::{PortDirection, QueueKind};

pub fn generate_processes_and_handles(ir: &WorkflowIR) -> (TokenStream, TokenStream, TokenStream) {
    let mut tokens = Vec::new();
    let mut fields = Vec::new();
    let mut instantiations = Vec::new();

    for (proc_name, proc_ir) in &ir.processes {
        let process_ident = format_ident!("{}_proc", proc_name.to_lowercase());
        let process_name_str = proc_name.clone();
        let interface_name = &proc_ir.interface_name;

        let ports_ident = format_ident!("{}_PORTS", proc_name.to_uppercase());

        // Look up the interface to get port specifications
        let mut port_specs = Vec::new();
        if let Some(iface) = ir.interfaces.get(interface_name) {
            for (port_name, port_ir) in &iface.ports {
                let name_str = port_name.clone();
                let dir_quote = match port_ir.direction {
                    PortDirection::In => quote! { vil_types::PortDirection::In },
                    PortDirection::Out => quote! { vil_types::PortDirection::Out },
                    PortDirection::Request => quote! { vil_types::PortDirection::Request },
                    PortDirection::Response => quote! { vil_types::PortDirection::Response },
                };
                let q_cap = port_ir.queue_spec.capacity;
                let q_kind = match port_ir.queue_spec.kind {
                    QueueKind::Spsc => quote! { vil_types::QueueKind::Spsc },
                    QueueKind::Mpmc => quote! { vil_types::QueueKind::Mpmc },
                };

                port_specs.push(quote! {
                    vil_types::PortSpec {
                        name: #name_str,
                        direction: #dir_quote,
                        queue: #q_kind,
                        capacity: #q_cap,
                        // Default properties for now, can be extracted from IR
                        backpressure: vil_types::BackpressurePolicy::Block,
                        transfer_mode: vil_types::TransferMode::LoanWrite,
                        boundary: vil_types::BoundaryKind::InterThreadLocal,
                        timeout_ms: None,
                        priority: vil_types::Priority::Normal,
                        delivery: vil_types::DeliveryGuarantee::BestEffort,
                        observability: vil_types::ObservabilitySpec {
                            tracing: false,
                            metrics: false,
                            lineage: false,
                            audit_sample_handoff: false,
                            latency_class: vil_types::LatencyClass::Normal,
                        },
                    }
                });
            }
        }

        tokens.push(quote! {
            static #ports_ident: &[vil_types::PortSpec] = &[ #(#port_specs),* ];

            let #process_ident = world.register_process(
                vil_types::ProcessSpec {
                    id: stringify!(#process_ident),
                    name: #process_name_str,
                    exec: vil_types::ExecClass::Thread,
                    cleanup: vil_types::CleanupPolicy::ReclaimOrphans,
                    ports: #ports_ident,
                    observability: vil_types::ObservabilitySpec {
                        tracing: false,
                        metrics: false,
                        lineage: false,
                        audit_sample_handoff: false,
                        latency_class: vil_types::LatencyClass::Normal,
                    },
                }
            ).expect("Failed to register process");
        });

        fields.push(quote! {
            pub #process_ident: vil_rt::handle::ProcessHandle,
        });

        instantiations.push(quote! {
            #process_ident,
        });
    }

    (
        quote! { #(#tokens)* },
        quote! { #(#fields)* },
        quote! { #(#instantiations)* },
    )
}
