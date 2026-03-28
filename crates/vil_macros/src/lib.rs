// =============================================================================
// vil_macros — Surface Syntax (Proc Macros)
// =============================================================================
// Developer-friendly entry point for VIL.
// Macros here ONLY parse the syntax tree (using `syn`) and generate
// initialization code / Dot Builder API calls.
// Semantic logic (layout validation, etc.) must NOT reside in this crate.
// =============================================================================

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput};

/// Macro to mark a struct as a `vil::message`.
/// Accepts optional `memory_class = VariantName` argument.
///
/// Examples:
///   `#[message]` — default PagedExchange
///   `#[message(memory_class = ControlHeap)]`
///   `#[message(memory_class = PinnedRemote)]`
#[proc_macro_attribute]
pub fn message(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse optional memory_class = VariantName from attr
    let attr_str = attr.to_string();
    let memory_class_tokens = if let Some(eq_pos) = attr_str.find("memory_class") {
        let after = &attr_str[eq_pos + 12..];
        let variant = after
            .split('=')
            .nth(1)
            .unwrap_or("")
            .trim()
            .trim_matches(',')
            .trim()
            .to_string();
        let valid = [
            "PagedExchange",
            "PinnedRemote",
            "ControlHeap",
            "LocalScratch",
        ];
        if valid.contains(&variant.as_str()) {
            let ident = syn::Ident::new(&variant, proc_macro2::Span::call_site());
            quote! { ::vil_sdk::types::MemoryClass::#ident }
        } else {
            quote! { ::vil_sdk::types::MemoryClass::PagedExchange }
        }
    } else {
        quote! { ::vil_sdk::types::MemoryClass::PagedExchange }
    };

    generate_semantic_type(
        item,
        quote! { ::vil_sdk::types::SemanticKind::Message },
        memory_class_tokens,
        vil_types::SemanticKind::Message,
    )
}

/// Macro to mark a struct as state machine data.
/// State may only travel on the Data Lane using LoanWrite/LoanRead.
#[proc_macro_attribute]
pub fn vil_state(_attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_semantic_type(
        item,
        quote! { ::vil_sdk::types::SemanticKind::State },
        quote! { ::vil_sdk::types::MemoryClass::PagedExchange },
        vil_types::SemanticKind::State,
    )
}

/// Macro to mark a struct as an immutable event log.
/// Events may travel on the Data Lane or Control Lane, supporting LoanWrite and Copy.
#[proc_macro_attribute]
pub fn vil_event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_semantic_type(
        item,
        quote! { ::vil_sdk::types::SemanticKind::Event },
        quote! { ::vil_sdk::types::MemoryClass::PagedExchange },
        vil_types::SemanticKind::Event,
    )
}

/// Macro to mark a struct as a structured fault.
/// Faults may only travel on the Control Lane and must use Copy.
#[proc_macro_attribute]
pub fn vil_fault(_attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_semantic_type(
        item,
        quote! { ::vil_sdk::types::SemanticKind::Fault },
        quote! { ::vil_sdk::types::MemoryClass::ControlHeap },
        vil_types::SemanticKind::Fault,
    )
}

/// Macro to mark a struct as a routing decision.
/// Decisions may only travel on the Trigger Lane.
#[proc_macro_attribute]
pub fn vil_decision(_attr: TokenStream, item: TokenStream) -> TokenStream {
    generate_semantic_type(
        item,
        quote! { ::vil_sdk::types::SemanticKind::Decision },
        quote! { ::vil_sdk::types::MemoryClass::ControlHeap },
        vil_types::SemanticKind::Decision,
    )
}

/// Internal helper: generates code for all semantic type macros.
fn generate_semantic_type(
    item: TokenStream,
    semantic_kind_tokens: proc_macro2::TokenStream,
    memory_class_tokens: proc_macro2::TokenStream,
    semantic_kind_ir: vil_types::SemanticKind,
) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    // --- Phase 1: Semantic Validation ---
    let mut builder =
        vil_ir::builder::MessageBuilder::new(&name_str).semantic_kind(semantic_kind_ir);
    let mut field_names = Vec::new();
    let mut field_types = Vec::new();

    if let syn::Data::Struct(ref data) = input.data {
        for field in &data.fields {
            let field_name = field
                .ident
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_else(|| "_".to_string());
            let ir_type = syn_type_to_ir(&field.ty);

            // For code generation
            field_names.push(field_name.clone());
            let type_str = quote!(#ir_type);
            field_types.push(type_str);

            builder = builder.add_field(field_name, ir_type);
        }
    } else if let syn::Data::Enum(_) = input.data {
        // Support for defining Faults (or other semantic types) as Enums.
        // We bypass Layout validation for enums for now since VASI compliance checks are struct-centric.
        // In a real system, we'd need a robust enum memory layout validator.
        builder = builder.layout(vil_types::LayoutProfile::Flat);
    }

    let msg_ir = builder.build();
    let mut workflow_ir = vil_ir::core::WorkflowIR::new("ValidationContext");
    workflow_ir
        .messages
        .insert(name_str.clone(), msg_ir.clone());

    let validator = vil_validate::LayoutLegalityPass;
    let report = vil_validate::ValidationPass::run(&validator, &workflow_ir);

    if report.has_errors() {
        let error_msg = report
            .diagnostics
            .iter()
            .map(|d| format!("[VIL {}] {}", d.code, d.message))
            .collect::<Vec<_>>()
            .join("\n");
        return syn::Error::new_spanned(name, error_msg)
            .to_compile_error()
            .into();
    }

    let mut is_stable = true;
    for field in &msg_ir.fields {
        if !vil_validate::layout::is_type_vasi_compliant(&field.ty) {
            is_stable = false;
            break;
        }
    }

    let fault_handler_impl = if semantic_kind_ir == vil_types::SemanticKind::Fault {
        quote! {
            impl ::vil_sdk::types::FaultHandler for #name {
                fn signal_error(&self) {
                    println!("[ControlLane] ERROR SIGNAL: {:?}", self);
                }
                fn control_abort(&self, session_id: u64) {
                    println!("[ControlLane] ABORTING Session {} due to {:?}", session_id, self);
                }
                fn degrade(&self, level: u8) {
                    println!("[ControlLane] SYSTEM DEGRADED to level {} due to {:?}", level, self);
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #[derive(Clone, Debug)]
        #input

        unsafe impl ::vil_sdk::types::Vasi for #name {}
        unsafe impl ::vil_sdk::types::PodLike for #name {}

        impl #name {
            /// Auto-generated by VIL semantic type macro
            pub fn get_message_builder() -> ::vil_sdk::ir::MessageBuilder {
                ::vil_sdk::ir::MessageBuilder::new(#name_str)
                    .layout(::vil_sdk::types::LayoutProfile::Relative)
                    #(.add_field(#field_names, #field_types))*
            }
        }

        impl ::vil_sdk::types::MessageContract for #name {
            const META: ::vil_sdk::types::MessageMeta = ::vil_sdk::types::MessageMeta {
                name: #name_str,
                layout: ::vil_sdk::types::LayoutProfile::Relative,
                transfer_caps: &[::vil_sdk::types::TransferMode::LoanWrite, ::vil_sdk::types::TransferMode::LoanRead],
                is_stable: #is_stable,
                semantic_kind: #semantic_kind_tokens,
                memory_class: #memory_class_tokens,
            };
        }

        #fault_handler_impl
    };

    TokenStream::from(expanded)
}

fn syn_type_to_ir(ty: &syn::Type) -> vil_ir::core::TypeRefIR {
    use vil_ir::core::TypeRefIR;
    let type_str = quote!(#ty).to_string().replace(" ", "");

    if type_str.contains("VSlice") {
        TypeRefIR::VSlice(Box::new(TypeRefIR::Unknown(type_str)))
    } else if type_str.contains("VRef") {
        TypeRefIR::VRef(Box::new(TypeRefIR::Unknown(type_str)))
    } else if [
        "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "f32", "f64", "bool",
    ]
    .contains(&type_str.as_str())
    {
        TypeRefIR::Primitive(type_str)
    } else {
        TypeRefIR::Unknown(type_str)
    }
}

/// `#[trace_hop]` — marks a process for hop latency tracing.
/// Place BEFORE `#[process]` on a struct. The #[process] macro detects this
/// and emits `.obs_trace_hop()` in the generated ProcessIR builder chain.
#[proc_macro_attribute]
pub fn trace_hop(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    // Inject a marker attribute that #[process] will detect
    let expanded = quote! {
        #[vil_obs_trace_hop]
        #input
    };
    TokenStream::from(expanded)
}

/// `#[latency_marker("label")]` — named latency label for dashboarding.
/// Place BEFORE `#[process]` on a struct. The #[process] macro detects this and
/// emits `.obs_latency_label("label")` in the generated builder chain.
#[proc_macro_attribute]
pub fn latency_marker(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let label_str = attr.to_string().replace('"', "");
    let label_str = label_str.trim().to_string();
    let expanded = quote! {
        #[vil_obs_latency_label(#label_str)]
        #input
    };
    TokenStream::from(expanded)
}

/// Internal marker attribute injected by `#[trace_hop]`.
/// Consumed and stripped by `#[process]`. Must be registered so rustc accepts it.
#[proc_macro_attribute]
pub fn vil_obs_trace_hop(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Internal marker attribute injected by `#[latency_marker]`.
/// Consumed and stripped by `#[process]`. Must be registered so rustc accepts it.
#[proc_macro_attribute]
pub fn vil_obs_latency_label(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Macro to mark a struct as a `vil::process`.
/// Accepts `interface="InterfaceName"` and `zone = ZoneName` parameters.
/// Detects `#[vil_obs_trace_hop]` and `#[vil_obs_latency_label = "..."]`.
#[proc_macro_attribute]
pub fn process(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let mut interface_name = name_str.clone() + "Interface";
    let mut trust_zone_tokens = quote! {};
    let mut trace_hop_tokens = quote! {};
    let mut latency_label_tokens = quote! {};

    let attr_string = attr.to_string();
    if attr_string.contains("interface") {
        if let Some(start) = attr_string.find('"') {
            if let Some(end) = attr_string[start + 1..].find('"') {
                interface_name = attr_string[start + 1..start + 1 + end].to_string();
            }
        }
    }

    // Parse `zone = ZoneName`
    if let Some(eq_pos) = attr_string.find("zone") {
        let after = &attr_string[eq_pos + 4..];
        let zone_str = after
            .split('=')
            .nth(1)
            .unwrap_or("")
            .trim()
            .trim_matches(',')
            .trim()
            .to_string();
        if [
            "NativeCore",
            "NativeTrusted",
            "WasmCapsule",
            "ExternalBoundary",
        ]
        .contains(&zone_str.as_str())
        {
            let zone_ident = syn::Ident::new(&zone_str, proc_macro2::Span::call_site());
            trust_zone_tokens = quote! {
                .trust_zone(::vil_sdk::types::TrustZone::#zone_ident)
            };
        }
    }

    // Parse `trace_hop` flag from #[process] attribute (e.g. #[process(trace_hop)])
    if attr_string.contains("trace_hop") {
        trace_hop_tokens = quote! { .obs_trace_hop() };
    }

    // Parse `latency = "label"` from #[process] attribute
    if let Some(lat_pos) = attr_string.find("latency") {
        let after = &attr_string[lat_pos + 7..];
        // Find the quoted label value
        if let Some(start) = after.find('"') {
            if let Some(end) = after[start + 1..].find('"') {
                let label = &after[start + 1..start + 1 + end];
                latency_label_tokens = quote! { .obs_latency_label(#label) };
            }
        }
    }

    // Also detect obs marker attributes injected by #[trace_hop] and #[latency_marker]
    let has_trace_hop = input.attrs.iter().any(|a| {
        a.path()
            .get_ident()
            .map(|id| id == "vil_obs_trace_hop")
            .unwrap_or(false)
    });
    if has_trace_hop {
        trace_hop_tokens = quote! { .obs_trace_hop() };
    }

    for a in &input.attrs {
        if a.path()
            .get_ident()
            .map(|id| id == "vil_obs_latency_label")
            .unwrap_or(false)
        {
            let ts = a.to_token_stream().to_string();
            if let Some(start) = ts.find('"') {
                if let Some(end) = ts[start + 1..].find('"') {
                    let label = &ts[start + 1..start + 1 + end];
                    latency_label_tokens = quote! { .obs_latency_label(#label) };
                }
            }
        }
    }

    // Strip marker attrs so rustc doesn't complain
    let mut clean = input.clone();
    clean.attrs.retain(|a| {
        let id = a.path().get_ident().map(|i| i.to_string());
        !matches!(
            id.as_deref(),
            Some("vil_obs_trace_hop") | Some("vil_obs_latency_label")
        )
    });

    let expanded = quote! {
        #clean

        impl #name {
            /// Auto-generated by #[process]
            pub fn get_process_builder() -> ::vil_sdk::ir::ProcessBuilder {
                ::vil_sdk::ir::ProcessBuilder::new(#name_str, #interface_name)
                    .exec_class(::vil_sdk::types::ExecClass::Thread)
                    #trust_zone_tokens
                    #trace_hop_tokens
                    #latency_label_tokens
            }
        }
    };

    TokenStream::from(expanded)
}

mod parser;
mod vil_error_derive;
use parser::WorkflowDef;

/// Macro for declaratively defining an entire workflow.
/// Translates the syntax tree into a set of Dot Builder API calls
/// from `vil_ir::builder::WorkflowBuilder`, then invokes
/// `generate_workflow_init` from `vil_codegen_rust`.
#[proc_macro]
pub fn vil_workflow(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as WorkflowDef);

    let workflow_name = &def.name;

    let hosts_calls = def.hosts.iter().map(|h| {
        let name = h.name.to_string();
        let address = &h.address;
        quote! {
            .add_host(#name, #address)
        }
    });

    let processes_calls = def.processes.iter().map(|p| {
        quote! {
            .add_process(#p::get_process_builder().build())
        }
    });

    let instances_ir = def.instances.iter().map(|i| {
        let name = &i.name;
        let mut iface_chain = quote! { #name.build_interface_ir() };
        let mut proc_chain = quote! { #name.build_process_ir() };

        if let Some(host) = &i.host {
            let host_str = host.to_string();
            iface_chain = quote! { #iface_chain.host_affinity(#host_str) };
            proc_chain = quote! { #proc_chain.host_affinity(#host_str) };
        }

        quote! {
            .add_interface(#iface_chain)
            .add_process(#proc_chain)
        }
    });

    let routes_calls = def.routes.iter().map(|r| {
        let src_proc = r.src_process.to_string();
        let src_port = r.src_port.to_string();
        let dst_proc = r.dst_process.to_string();
        let dst_port = r.dst_port.to_string();
        let t_mode = &r.transfer_mode;
        
        if let Some(transport) = &r.transport {
            let trans_str = transport.to_string();
            quote! {
                .route_ext(#src_proc, #src_port, #dst_proc, #dst_port, vil_sdk::types::TransferMode::#t_mode, Some(#trans_str.to_string()))
            }
        } else {
            quote! {
                .route_ext(#src_proc, #src_port, #dst_proc, #dst_port, vil_sdk::types::TransferMode::#t_mode, None)
            }
        }
    });

    let failovers_calls = def.failovers.iter().map(|f| {
        let src = f.source.to_string();
        let condition = f.condition.to_string();

        let target = if f.target == "retry" {
            let attempts = f
                .retry_attempts
                .as_ref()
                .map(|a| a.base10_parse::<u32>().unwrap_or(3))
                .unwrap_or(3);
            let backoff = f
                .retry_backoff
                .as_ref()
                .map(|b| b.base10_parse::<u64>().unwrap_or(100))
                .unwrap_or(100);
            format!("retry({}, {}ms)", attempts, backoff)
        } else {
            f.target.to_string()
        };

        let strategy = f
            .strategy
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "ImmediateBackup".to_string());

        quote! {
            .failover(#src, #target, #condition, #strategy)
        }
    });

    let mut registration_calls = Vec::new();
    let mut generated_handles = Vec::new(); // to return the handles if needed

    for i in &def.instances {
        let i_name = &i.name;
        let handle_name = syn::Ident::new(&format!("{}_handle", i_name), i_name.span());
        registration_calls.push(quote! {
            let #handle_name = world.register_process(#i_name.build_spec()).expect("Failed to register process");
        });
        generated_handles.push(handle_name);
    }

    let mut wiring_calls = Vec::new();
    for r in &def.routes {
        // If the src or dst is an instance, use its handle.
        // For standard `processes` nodes, this macro assumes we are just generating IR in this PoC unless we instantiated them.
        let is_src_instance = def.instances.iter().any(|i| i.name == r.src_process);
        let is_dst_instance = def.instances.iter().any(|i| i.name == r.dst_process);

        if is_src_instance && is_dst_instance {
            let src_handle =
                syn::Ident::new(&format!("{}_handle", r.src_process), r.src_process.span());
            let dst_handle =
                syn::Ident::new(&format!("{}_handle", r.dst_process), r.dst_process.span());
            let src_port = r.src_port.to_string();
            let dst_port = r.dst_port.to_string();

            wiring_calls.push(quote! {
                world.connect(
                    #src_handle.port_id(#src_port).expect("src port not found"),
                    #dst_handle.port_id(#dst_port).expect("dst port not found")
                );
            });
        }
    }

    // Return the expression block
    let expanded = quote! {
        {
            let __ir = vil_sdk::ir::WorkflowBuilder::new(#workflow_name)
                #(#hosts_calls)*
                #(#processes_calls)*
                #(#instances_ir)*
                #(#routes_calls)*
                #(#failovers_calls)*
                .infer_transfers()
                .build();

            #(#registration_calls)*
            #(#wiring_calls)*

            (__ir, (#(#generated_handles),*))
        }
    };

    TokenStream::from(expanded)
}

// =============================================================================
// Phase 2: VilModel & VilError derive macros
// =============================================================================

/// Derive macro that generates a `VilModel` impl for a struct.
///
/// The struct **must** also derive `Serialize`, `Deserialize`, and `Clone`.
/// This macro only generates the `VilModel` trait implementation — it does
/// not add those derives automatically.
///
/// # Example
///
/// ```ignore
/// use serde::{Serialize, Deserialize};
/// use vil_macros::VilModel;
///
/// #[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// struct Task {
///     id: u64,
///     title: String,
///     done: bool,
/// }
/// ```
#[proc_macro_derive(VilModel)]
pub fn derive_vil_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl ::vil_server_core::model::VilModel for #name {
            fn from_shm_bytes(bytes: &[u8]) -> ::core::result::Result<Self, ::vil_server_core::VilError> {
                ::vil_json::from_slice(bytes)
                    .map_err(|e| ::vil_server_core::VilError::bad_request(e.to_string()))
            }

            fn to_json_bytes(&self) -> ::core::result::Result<::bytes::Bytes, ::vil_server_core::VilError> {
                ::vil_json::to_bytes(self)
                    .map_err(|e| ::vil_server_core::VilError::internal(e.to_string()))
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro that generates `Display`, `Error`, and `From<Self> for VilError`
/// impls for an enum whose variants carry `#[vil_error(status = NNN)]` attributes.
///
/// # Status Code Mapping
///
/// | Code | VilError factory        |
/// |------|---------------------------|
/// | 400  | `bad_request()`           |
/// | 401  | `unauthorized()`          |
/// | 403  | `forbidden()`             |
/// | 404  | `not_found()`             |
/// | 422  | `validation()`            |
/// | 429  | `rate_limited()`          |
/// | 500  | `internal()`              |
/// | 503  | `service_unavailable()`   |
/// | other| `internal()`              |
///
/// # Example
///
/// ```ignore
/// use vil_macros::VilError;
///
/// #[derive(Debug, VilError)]
/// enum TaskError {
///     #[vil_error(status = 404)]
///     NotFound { id: u64 },
///     #[vil_error(status = 422)]
///     InvalidTitle,
///     #[vil_error(status = 500)]
///     DatabaseError(String),
/// }
/// ```
#[proc_macro_derive(VilError, attributes(vil_error))]
pub fn derive_vil_error(input: TokenStream) -> TokenStream {
    vil_error_derive::derive_vil_error_impl(input)
}

// =============================================================================
// Tier B: AI Semantic Derive Macros
// =============================================================================
// These are lightweight alternatives to Tier A (#[vil_event], #[vil_state]).
// They work with any Serialize type (String, Vec, HashMap — dynamic sizes OK).
// No SHM/VASI requirement — just semantic classification + observability.

/// Derive macro: marks a struct as an AI Event (Tier B).
///
/// AI Events are immutable audit records that flow on the Data Lane.
/// Used for: LLM responses, RAG query results, agent completions.
///
/// ```ignore
/// #[derive(Clone, Debug, Serialize, Deserialize, VilAiEvent)]
/// struct LlmResponseEvent {
///     provider: String,
///     model: String,
///     tokens: u32,
/// }
/// ```
#[proc_macro_derive(VilAiEvent)]
pub fn derive_vil_ai_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        impl ::vil_server_core::plugin_system::semantic::AiSemantic for #name {
            fn semantic_kind() -> ::vil_server_core::plugin_system::semantic::AiSemanticKind {
                ::vil_server_core::plugin_system::semantic::AiSemanticKind::Event
            }
            fn lane() -> ::vil_server_core::plugin_system::semantic::AiLane {
                ::vil_server_core::plugin_system::semantic::AiLane::Data
            }
            fn type_name() -> &'static str {
                #name_str
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro: marks a struct as an AI Fault (Tier B).
///
/// AI Faults are error signals that flow on the Control Lane.
/// Used for: LLM failures, retrieval errors, agent timeouts.
///
/// ```ignore
/// #[derive(Clone, Debug, Serialize, Deserialize, VilAiFault)]
/// struct LlmFault {
///     provider: String,
///     error_type: String,
///     message: String,
/// }
/// ```
#[proc_macro_derive(VilAiFault)]
pub fn derive_vil_ai_fault(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        impl ::vil_server_core::plugin_system::semantic::AiSemantic for #name {
            fn semantic_kind() -> ::vil_server_core::plugin_system::semantic::AiSemanticKind {
                ::vil_server_core::plugin_system::semantic::AiSemanticKind::Fault
            }
            fn lane() -> ::vil_server_core::plugin_system::semantic::AiLane {
                ::vil_server_core::plugin_system::semantic::AiLane::Control
            }
            fn type_name() -> &'static str {
                #name_str
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro: marks a struct as AI State (Tier B).
///
/// AI State is mutable tracked state on the Data Lane.
/// Used for: usage counters, index stats, memory state.
#[proc_macro_derive(VilAiState)]
pub fn derive_vil_ai_state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        impl ::vil_server_core::plugin_system::semantic::AiSemantic for #name {
            fn semantic_kind() -> ::vil_server_core::plugin_system::semantic::AiSemanticKind {
                ::vil_server_core::plugin_system::semantic::AiSemanticKind::State
            }
            fn lane() -> ::vil_server_core::plugin_system::semantic::AiLane {
                ::vil_server_core::plugin_system::semantic::AiLane::Data
            }
            fn type_name() -> &'static str {
                #name_str
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro: marks a struct as AI Decision (Tier B).
///
/// AI Decisions are routing signals on the Trigger Lane.
/// Used for: agent routing, model selection, A/B test assignment.
#[proc_macro_derive(VilAiDecision)]
pub fn derive_vil_ai_decision(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        impl ::vil_server_core::plugin_system::semantic::AiSemantic for #name {
            fn semantic_kind() -> ::vil_server_core::plugin_system::semantic::AiSemanticKind {
                ::vil_server_core::plugin_system::semantic::AiSemanticKind::Decision
            }
            fn lane() -> ::vil_server_core::plugin_system::semantic::AiLane {
                ::vil_server_core::plugin_system::semantic::AiLane::Trigger
            }
            fn type_name() -> &'static str {
                #name_str
            }
        }
    };

    TokenStream::from(expanded)
}
