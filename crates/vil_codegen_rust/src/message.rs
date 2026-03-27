// =============================================================================
// message.rs — Code Generator for MessageIR
// =============================================================================

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use vil_ir::core::WorkflowIR;

/// Generates a TokenStream containing Rust struct definitions and trait
/// implementations based on the MessageIR collection from a Workflow.
pub fn generate_messages(ir: &WorkflowIR) -> TokenStream {
    let mut messages = Vec::new();

    for (name, msg_ir) in &ir.messages {
        let ident = format_ident!("{}", name);
        let layout_ident = match msg_ir.layout {
            vil_types::LayoutProfile::Flat => quote! { vil_types::LayoutProfile::Flat },
            vil_types::LayoutProfile::Relative => quote! { vil_types::LayoutProfile::Relative },
            vil_types::LayoutProfile::External => quote! { vil_types::LayoutProfile::External },
        };

        // In production, message fields would also be generated.
        // Currently the Semantic IR does not store field AST definitions
        // (only struct names for the routing pass), so we generate opaque structs
        // or let the developer define actual structs via derive macros.
        // Codegen generates MessageContract implementations for structs
        // previously defined by `#[vil::message]`.
        
        // Output a blanket implementation for `MessageContract`
        // wrapping the architecture's meta information.
        let name_str = name.clone();
        
        // The `#[vil::message]` proc-macro in `vil_macros` emits the Rust
        // struct, while this Workflow Codegen injects the Meta Contract
        // according to workflow topology.
        // For automatic boilerplate:
        messages.push(quote! {
            impl vil_types::MessageContract for #ident {
                const META: vil_types::MessageMeta = vil_types::MessageMeta {
                    name: #name_str,
                    layout: #layout_ident,
                    // TODO: Implement TransferCaps extraction based on workflow usage
                    transfer_caps: &[vil_types::TransferMode::LoanWrite, vil_types::TransferMode::LoanRead], 
                    is_stable: false,
                    semantic_kind: vil_types::SemanticKind::Message,
                    memory_class: vil_types::MemoryClass::PagedExchange,
                };
            }
        });
    }

    quote! {
        #(#messages)*
    }
}
