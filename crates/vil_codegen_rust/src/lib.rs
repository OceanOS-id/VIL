// =============================================================================
// vil_codegen_rust — Semantic IR to Rust Code Generator
// =============================================================================
// Transforms Semantic IR (`WorkflowIR`) into pure Rust TokenStream
// using `quote` and `proc-macro2`.
// Output is runtime initialization boilerplate for `vil_rt`:
// process registration, port creation, and topology wiring.
// =============================================================================

pub mod message;
pub mod process;
pub mod workflow;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use vil_ir::core::WorkflowIR;

/// Main function to transform WorkflowIR into Rust runtime initialization code.
pub fn generate_workflow(ir: &WorkflowIR) -> TokenStream {
    let workflow_name_str = &ir.name;
    let fn_ident = format_ident!("init_workflow_{}", workflow_name_str.to_lowercase());

    // 1. Generate Messages (Structs and Meta Contracts)
    let messages_code = message::generate_messages(ir);

    // 2. Generate Process Inits (PortSpecs and ProcessSpecs)
    let (processes_code, handle_fields, handle_instantiations) =
        process::generate_processes_and_handles(ir);

    // 3. Generate Route Wiring
    let routes_code = workflow::generate_routes(ir);

    let struct_ident = format_ident!("{}Handles", workflow_name_str);

    // Assembly function
    quote! {
        #messages_code

        pub struct #struct_ident {
            #handle_fields
        }

        pub fn #fn_ident(world: &vil_rt::VastarRuntimeWorld) -> #struct_ident {
            println!("Initializing Workflow Architecture: {}", #workflow_name_str);

            // --- Register Processes ---
            #processes_code

            // --- Apply Routes ---
            #routes_code

            #struct_ident {
                #handle_instantiations
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vil_ir::builder::{InterfaceBuilder, ProcessBuilder, WorkflowBuilder};
    use vil_types::TransferMode;

    #[test]
    fn test_codegen_basic() {
        let ir = WorkflowBuilder::new("TestFlow")
            .add_interface(
                InterfaceBuilder::new("Iface")
                    .out_port("tx", "Msg")
                    .done()
                    .in_port("rx", "Msg")
                    .done()
                    .build(),
            )
            .add_process(ProcessBuilder::new("A", "Iface").build())
            .add_process(ProcessBuilder::new("B", "Iface").build())
            .route("A", "tx", "B", "rx", TransferMode::LoanWrite)
            .build();

        let token_stream = generate_workflow(&ir);
        let result = token_stream.to_string();

        let result: String = result.chars().filter(|c| !c.is_whitespace()).collect();

        assert!(result.contains("init_workflow_testflow"));
        assert!(result.contains("TestFlowHandles"));
        assert!(result.contains("world.register_process"));
        assert!(result.contains("world.connect"));
    }
}
