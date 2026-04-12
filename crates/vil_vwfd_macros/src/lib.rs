//! # vil_vwfd_macros — Declarative VWFD Workflow Macro
//!
//! Generates VWFD YAML + Rust module from macro syntax.
//!
//! ```ignore
//! vil_vwfd! {
//!     id: "create-order",
//!     trigger: webhook("/orders") { method: POST, response_mode: buffered, end_activity: "respond" },
//!     activities: {
//!         validate => connector("vastar.http") {
//!             operation: post,
//!             input: { url: literal("http://api.example.com"), body: cel("trigger_payload") },
//!             output: "validated",
//!         },
//!         respond => end_trigger("trigger") {
//!             response: cel(r#"{"result": validated}"#),
//!         },
//!     },
//!     flow: trigger -> validate -> respond -> end,
//! }
//! ```
//!
//! Generates module `create_order` with:
//! - `VWFD_YAML: &str` — full VWFD YAML
//! - `metadata()` — WorkflowMeta
//! - `compile()` → VilwGraph
//! - `register(&mut WorkflowRegistry)` — register in handler

mod parser;
mod codegen;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Declarative VWFD workflow definition.
///
/// Same VWFD format compatible with VFlow. Generates YAML + compile helper.
#[proc_macro]
pub fn vil_vwfd(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as parser::VwfdMacroDef);
    let output = codegen::generate(&def);
    output.into()
}
