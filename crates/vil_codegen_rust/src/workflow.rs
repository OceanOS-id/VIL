// =============================================================================
// workflow.rs — Code Generator for RouteIR Topology 
// =============================================================================

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use vil_ir::core::WorkflowIR;

pub fn generate_routes(ir: &WorkflowIR) -> TokenStream {
    let mut tokens = Vec::new();

    for route in &ir.routes {
        let from_proc_ident = format_ident!("{}_proc", route.from_process.to_lowercase());
        let to_proc_ident = format_ident!("{}_proc", route.to_process.to_lowercase());
        let from_port_str = &route.from_port;
        let to_port_str = &route.to_port;

        tokens.push(quote! {
            {
                let p_out = #from_proc_ident.port_id(#from_port_str).expect("Tx port not found");
                let p_in  = #to_proc_ident.port_id(#to_port_str).expect("Rx port not found");
                world.connect(p_out, p_in);
            }
        });
    }

    quote! {
        #(#tokens)*
    }
}
