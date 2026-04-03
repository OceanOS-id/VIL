//! # vil_server_macros
//!
//! Procedural macros for VIL server handlers and SSE events.
//!
//! ## Macros
//!
//! - `#[vil_handler]` — Wraps an async handler with RequestId injection,
//!   tracing span generation, and automatic response/error mapping.
//! - `#[derive(VilSseEvent)]` — Derives SSE event conversion and broadcast
//!   methods for a struct.
//! - `#[derive(VilWsEvent)]` — Derives WebSocket event conversion, broadcast,
//!   and topic-based routing methods for a struct.
//! - `#[vil_endpoint]` — Marks an async fn as a VX endpoint with tracing
//!   and optional execution class dispatch (AsyncTask, BlockingTask, DedicatedThread).
//! - `vil_app!` — Declarative DSL for defining a VX application with
//!   services, endpoints, and configuration.
//! - `#[vil_service_state]` — Marks a struct as VX managed service state
//!   with optional storage backend (`PrivateHeap` or `SharedShm`).
//! - `#[vil_service]` — Module-level attribute that generates a service
//!   factory function, name/prefix constants, and mesh requirements.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, DeriveInput, FnArg, Ident, ItemFn, ItemMod, LitInt, LitStr, Pat, ReturnType,
    Token, Type,
};

/// Attribute macro that wraps an async handler function with:
///
/// 1. **RequestId auto-injection** as the first parameter
/// 2. **Tracing span** auto-generation using the function name
/// 3. **Return type wrapping** — the wrapper returns `axum::response::Response`
/// 4. **Error mapping** via `Into<VilError>` for `Result` return types
///
/// # Usage
///
/// For handlers that return `Result<T, E>`:
///
/// ```ignore
/// #[vil_handler]
/// async fn get_user(id: Path<u64>) -> Result<User, AppError> {
///     let user = db::find_user(*id).await?;
///     Ok(user)
/// }
/// ```
///
/// For handlers that return a plain value:
///
/// ```ignore
/// #[vil_handler]
/// async fn health_check() -> &'static str {
///     "ok"
/// }
/// ```
///
/// The macro renames the original function to `__vil_inner_<name>` and
/// generates a public wrapper that:
/// - Accepts `RequestId` as its first parameter (for Axum extractor injection)
/// - Opens a tracing `info_span` tagged with the request id
/// - Delegates to the inner function
/// - Maps the result into an `axum::response::Response`
/// Parse `#[vil_handler]` or `#[vil_handler(shm)]` attribute.
struct VilHandlerAttr {
    shm_mode: bool,
}

impl Parse for VilHandlerAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(VilHandlerAttr { shm_mode: false });
        }
        let ident: Ident = input.parse()?;
        if ident == "shm" {
            Ok(VilHandlerAttr { shm_mode: true })
        } else {
            Err(syn::Error::new_spanned(ident, "expected `shm`"))
        }
    }
}

/// Check if a type is a known axum/VIL extractor (should not be rewritten).
fn is_vil_extractor(ty: &Type) -> bool {
    let type_str = quote!(#ty).to_string().replace(' ', "");
    let known = [
        "Path",
        "Query",
        "State",
        "Extension",
        "Json",
        "ShmSlice",
        "ShmContext",
        "ServiceCtx",
        "TriLaneCtx",
        "TriLaneRouter",
        "IngressBridge",
        "RequestId",
        "Bytes",
        "String",
        "&str",
        "u8",
        "u16",
        "u32",
        "u64",
        "i32",
        "i64",
        "bool",
        "usize",
        "Request",
    ];
    known
        .iter()
        .any(|k| type_str.starts_with(k) || type_str.contains(&format!("::{}", k)))
}

#[proc_macro_attribute]
pub fn vil_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    let handler_attr = parse_macro_input!(attr as VilHandlerAttr);
    let input = parse_macro_input!(item as ItemFn);
    let vis = &input.vis;
    let name = &input.sig.ident;
    let inner_name = format_ident!("__vil_inner_{}", name);
    let body = &input.block;
    let asyncness = &input.sig.asyncness;
    let return_type = &input.sig.output;
    let name_str = name.to_string();

    // In SHM mode: rewrite unknown body params to ShmSlice, auto-inject ServiceCtx
    let (inputs, extra_wrapper_params) = if handler_attr.shm_mode {
        let mut rewritten: syn::punctuated::Punctuated<FnArg, Token![,]> =
            syn::punctuated::Punctuated::new();
        let mut has_ctx = false;

        for arg in input.sig.inputs.iter() {
            if let FnArg::Typed(pat_type) = arg {
                // Check if this is ServiceCtx
                let ty_str = quote!(#pat_type.ty).to_string();
                if ty_str.contains("ServiceCtx") {
                    has_ctx = true;
                }

                if is_vil_extractor(&pat_type.ty) {
                    rewritten.push(arg.clone());
                } else {
                    // Rewrite unknown body type → ShmSlice
                    let pat = &pat_type.pat;
                    let new_arg: FnArg = syn::parse_quote! {
                        #pat: ::vil_server::__private::ShmSlice
                    };
                    rewritten.push(new_arg);
                }
            } else {
                rewritten.push(arg.clone());
            }
        }

        // Auto-inject ServiceCtx if not already present
        let extra = if !has_ctx {
            quote! { __vil_ctx: ::vil_server::__private::ServiceCtx, }
        } else {
            quote! {}
        };

        (rewritten, extra)
    } else {
        (input.sig.inputs.clone(), quote! {})
    };

    // Collect parameter patterns for call forwarding (from ORIGINAL inputs).
    // Handles both simple idents (`ctx`) and destructuring patterns (`Query(filter)`, `Path(id)`).
    let param_pats: Vec<proc_macro2::TokenStream> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                let pat = &pat_type.pat;
                Some(quote! { #pat })
            } else {
                None
            }
        })
        .collect();
    // Alias for backward compatibility in quote! blocks below
    let param_names = &param_pats;

    // Collect rewritten parameter names+types for wrapper signature
    let wrapper_params: Vec<_> = inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                let pat = &pat_type.pat;
                let ty = &pat_type.ty;
                Some(quote! { #pat: #ty })
            } else {
                None
            }
        })
        .collect();

    // Determine if return type is Result<T, E>
    let is_result = match return_type {
        ReturnType::Type(_, ty) => {
            if let Type::Path(type_path) = &**ty {
                type_path
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident == "Result")
                    .unwrap_or(false)
            } else {
                false
            }
        }
        ReturnType::Default => false,
    };

    // Detect if return type already contains VilResponse (passthrough mode).
    // Handles: VilResponse<T>, Result<VilResponse<T>, E>, Result<impl IntoResponse, E>
    // In passthrough mode, the macro does NOT re-wrap with VilResponse::ok() —
    // it calls .into_response() directly, preserving the handler's HTTP status code.
    let returns_vil_response = {
        let ty_str = quote!(#return_type).to_string().replace(' ', "");
        ty_str.contains("VilResponse")
            || ty_str.contains("IntoResponse")
            || ty_str.contains("axum::response::Response")
            || ty_str.contains("AxumResponse")
            || ty_str.contains("Response<")
    };

    // Inline access log emission block — avoids depending on macro_export path resolution.
    // Uses ::vil_server::__private::vil_log for all types, which is re-exported from vil_server.
    let emit_access_log = quote! {
        {
            use ::vil_server::__private::vil_log::emit::ring::{try_global_ring, level_enabled};
            use ::vil_server::__private::vil_log::types::{
                LogSlot, VilLogHeader, LogLevel, LogCategory, AccessPayload,
            };
            use ::vil_server::__private::vil_log::dict::register_str;

            if level_enabled(LogLevel::Info as u8) {
                if let Some(__vil_ring) = try_global_ring() {
                    let __vil_ts = {
                        use ::std::time::{SystemTime, UNIX_EPOCH};
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos() as u64
                    };

                    let mut __vil_slot = LogSlot::default();
                    __vil_slot.header = VilLogHeader {
                        timestamp_ns: __vil_ts,
                        level:        LogLevel::Info as u8,
                        category:     LogCategory::Access as u8,
                        version:      1,
                        service_hash: register_str(module_path!()),
                        handler_hash: register_str(#name_str),
                        process_id:   ::std::process::id() as u64,
                        ..VilLogHeader::default()
                    };

                    let __vil_payload = AccessPayload {
                        status_code:    __vil_status,
                        duration_us:    __vil_elapsed.as_micros() as u32,
                        route_hash:     register_str(#name_str),
                        path_hash:      register_str(#name_str),
                        session_id:     register_str(&__vil_rid.0) as u64,
                        ..AccessPayload::default()
                    };

                    let __vil_payload_bytes = unsafe {
                        ::std::slice::from_raw_parts(
                            &__vil_payload as *const _ as *const u8,
                            ::std::mem::size_of::<AccessPayload>().min(192),
                        )
                    };
                    let __vil_copy_len = __vil_payload_bytes.len().min(192);
                    __vil_slot.payload[..__vil_copy_len]
                        .copy_from_slice(&__vil_payload_bytes[..__vil_copy_len]);

                    let _ = __vil_ring.try_push(__vil_slot);
                }
            }
        }
    };

    let wrapper_body = if returns_vil_response && is_result {
        // ── Passthrough mode (Result<VilResponse<T>, E>) ──
        // Handler already wraps response with VilResponse — don't re-wrap.
        // This preserves custom HTTP status codes (201 Created, 204 No Content, etc.)
        quote! {
            let _span = ::vil_server::__private::tracing::info_span!(#name_str, request_id = %__vil_rid).entered();
            let __vil_start = ::std::time::Instant::now();
            let __vil_resp = match #inner_name(#(#param_names),*).await {
                Ok(data) => {
                    // VilResponse/Response already implements IntoResponse — passthrough
                    ::vil_server::__private::axum::response::IntoResponse::into_response(data)
                }
                Err(e) => {
                    let vil_err: ::vil_server::__private::VilError = e.into();
                    ::vil_server::__private::axum::response::IntoResponse::into_response(vil_err)
                }
            };
            let __vil_elapsed = __vil_start.elapsed();
            let __vil_status = __vil_resp.status().as_u16();
            #emit_access_log
            __vil_resp
        }
    } else if returns_vil_response {
        // ── Passthrough mode (VilResponse<T> directly, no Result) ──
        quote! {
            let _span = ::vil_server::__private::tracing::info_span!(#name_str, request_id = %__vil_rid).entered();
            let __vil_start = ::std::time::Instant::now();
            let data = #inner_name(#(#param_names),*).await;
            let __vil_resp = ::vil_server::__private::axum::response::IntoResponse::into_response(data);
            let __vil_elapsed = __vil_start.elapsed();
            let __vil_status = __vil_resp.status().as_u16();
            #emit_access_log
            __vil_resp
        }
    } else if is_result {
        // ── Standard mode (Result<T: Serialize, E>) ──
        // Wrap Ok(data) with VilResponse::ok() (always 200)
        quote! {
            let _span = ::vil_server::__private::tracing::info_span!(#name_str, request_id = %__vil_rid).entered();
            let __vil_start = ::std::time::Instant::now();
            let __vil_resp = match #inner_name(#(#param_names),*).await {
                Ok(data) => {
                    ::vil_server::__private::axum::response::IntoResponse::into_response(
                        ::vil_server::__private::response::VilResponse::ok(data)
                    )
                }
                Err(e) => {
                    let vil_err: ::vil_server::__private::VilError = e.into();
                    ::vil_server::__private::axum::response::IntoResponse::into_response(vil_err)
                }
            };
            let __vil_elapsed = __vil_start.elapsed();
            let __vil_status = __vil_resp.status().as_u16();
            #emit_access_log
            __vil_resp
        }
    } else {
        // ── Standard mode (plain T: Serialize) ──
        // Wrap with VilResponse::ok() (always 200)
        quote! {
            let _span = ::vil_server::__private::tracing::info_span!(#name_str, request_id = %__vil_rid).entered();
            let __vil_start = ::std::time::Instant::now();
            let data = #inner_name(#(#param_names),*).await;
            let __vil_resp = ::vil_server::__private::axum::response::IntoResponse::into_response(
                ::vil_server::__private::response::VilResponse::ok(data)
            );
            let __vil_elapsed = __vil_start.elapsed();
            let __vil_status = __vil_resp.status().as_u16();
            #emit_access_log
            __vil_resp
        }
    };

    // Original inputs for inner function (user's original code)
    let original_inputs = &input.sig.inputs;

    let expanded = quote! {
        // Inner function preserves the user's original signature
        #asyncness fn #inner_name(#original_inputs) #return_type #body

        // Public wrapper with RequestId + (optional ServiceCtx in shm mode) injection
        #vis async fn #name(
            __vil_rid: ::vil_server::__private::RequestId,
            #extra_wrapper_params
            #(#wrapper_params),*
        ) -> ::vil_server::__private::axum::response::Response {
            #wrapper_body
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro that generates SSE event helpers for a struct.
///
/// Adds two methods:
/// - `to_sse_event(&self)` — serializes the struct into an `axum::response::sse::Event`
/// - `broadcast(&self, hub)` — broadcasts the serialized event to all subscribers
///
/// # Attributes
///
/// Use `#[sse_event(topic = "...")]` on the struct to set a custom event/topic name.
/// If omitted, the topic defaults to the lowercase struct name.
///
/// # Example
///
/// ```ignore
/// #[derive(Serialize, VilSseEvent)]
/// #[sse_event(topic = "user_update")]
/// struct UserUpdated {
///     user_id: u64,
///     name: String,
/// }
/// ```
#[proc_macro_derive(VilSseEvent, attributes(sse_event))]
pub fn derive_vil_sse_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Default topic is lowercase struct name
    let mut topic = name.to_string().to_lowercase();

    // Parse #[sse_event(topic = "...")] attribute if present
    for attr in &input.attrs {
        if attr.path().is_ident("sse_event") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("topic") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    topic = lit.value();
                    Ok(())
                } else {
                    Err(meta.error("unsupported sse_event attribute, expected `topic`"))
                }
            });
        }
    }

    let expanded = quote! {
        impl #name {
            /// Convert this value into an SSE `Event` for streaming.
            ///
            /// The event name is set to the configured topic and the data
            /// payload is the JSON serialization of `self`.
            pub fn to_sse_event(
                &self,
            ) -> ::core::result::Result<::vil_server::__private::axum::response::sse::Event, ::std::convert::Infallible>
            {
                let data = ::serde_json::to_string(self).unwrap_or_default();
                Ok(::vil_server::__private::axum::response::sse::Event::default()
                    .event(#topic)
                    .data(data))
            }

            /// Broadcast this event to all subscribers of the given SSE hub.
            ///
            /// Serializes `self` to JSON and sends it on the configured topic.
            pub fn broadcast(&self, hub: &::vil_server::__private::streaming::SseHub) {
                let data = ::serde_json::to_string(self).unwrap_or_default();
                hub.broadcast(#topic, data);
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro that generates WebSocket event helpers for a struct.
///
/// Adds the following methods:
/// - `to_ws_message(&self)` — serializes the struct into an `axum::extract::ws::Message::Text`
/// - `from_ws_message(msg)` — deserializes from a WebSocket `Message::Text`
/// - `ws_topic()` — returns the configured topic name
/// - `broadcast(&self, hub)` — broadcasts the serialized event to all subscribers in the WsHub
///
/// # Attributes
///
/// Use `#[ws_event(topic = "...")]` on the struct to set a custom topic name.
/// If omitted, the topic defaults to the lowercase struct name.
///
/// # Example
///
/// ```ignore
/// #[derive(Serialize, Deserialize, VilWsEvent)]
/// #[ws_event(topic = "chat")]
/// struct ChatMessage {
///     sender: String,
///     content: String,
/// }
/// ```
#[proc_macro_derive(VilWsEvent, attributes(ws_event))]
pub fn derive_vil_ws_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Default topic is lowercase struct name
    let mut topic = name.to_string().to_lowercase();

    // Parse #[ws_event(topic = "...")] attribute if present
    for attr in &input.attrs {
        if attr.path().is_ident("ws_event") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("topic") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    topic = lit.value();
                    Ok(())
                } else {
                    Err(meta.error("unsupported ws_event attribute, expected `topic`"))
                }
            });
        }
    }

    let expanded = quote! {
        impl #name {
            /// Serialize this value into a WebSocket `Message::Text` (JSON payload).
            pub fn to_ws_message(&self) -> ::vil_server::__private::axum::extract::ws::Message {
                let json = ::serde_json::to_string(self).unwrap_or_default();
                ::vil_server::__private::axum::extract::ws::Message::Text(json)
            }

            /// Deserialize from a WebSocket `Message::Text`.
            ///
            /// Returns an error if the message is not a `Text` variant or if
            /// JSON deserialization fails.
            pub fn from_ws_message(
                msg: &::vil_server::__private::axum::extract::ws::Message,
            ) -> ::core::result::Result<Self, ::serde_json::Error>
            where
                Self: ::serde::de::DeserializeOwned,
            {
                match msg {
                    ::vil_server::__private::axum::extract::ws::Message::Text(text) => {
                        ::serde_json::from_str(text)
                    }
                    _ => Err(::serde::de::Error::custom("expected WebSocket Text message")),
                }
            }

            /// Get the topic name for this WebSocket event type.
            pub fn ws_topic() -> &'static str {
                #topic
            }

            /// Broadcast this event to all subscribers of the configured topic
            /// in the given `WsHub`.
            pub fn broadcast(&self, hub: &::vil_server::__private::streaming::WsHub) {
                let json = ::serde_json::to_string(self).unwrap_or_default();
                hub.broadcast(#topic, json);
            }
        }
    };

    TokenStream::from(expanded)
}

// =============================================================================
// vil_endpoint — VX endpoint attribute macro
// =============================================================================

/// Parsed attributes for `#[vil_endpoint(exec = ...)]`.
struct EndpointAttr {
    exec_class: Option<Ident>,
}

impl Parse for EndpointAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(EndpointAttr { exec_class: None });
        }

        let mut exec_class = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            if key == "exec" {
                let _eq: Token![=] = input.parse()?;
                exec_class = Some(input.parse::<Ident>()?);
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "unknown vil_endpoint attribute, expected `exec`",
                ));
            }

            // consume optional trailing comma
            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
            }
        }

        Ok(EndpointAttr { exec_class })
    }
}

/// Returns `true` if the given type should NOT be auto-wrapped with `Json<T>`.
///
/// Known extractors (e.g., `Path`, `Query`, `Json`, `State`), primitive types,
/// and non-path types (references, tuples) are all considered "known" and will
/// be left as-is by the parameter rewriting logic.
fn is_known_extractor(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let name = segment.ident.to_string();
            matches!(
                name.as_str(),
                // Axum extractors
                "Json" | "Path" | "Query" | "State" | "Extension"
                // VLang extractors
                | "ShmSlice" | "ShmContext" | "ServiceCtx"
                // HTTP types
                | "HeaderMap" | "StatusCode" | "Method" | "Request"
                | "WebSocketUpgrade"
                // VLang routing types
                | "IngressBridge" | "TriLaneRouter"
                // Standard / primitive types — never body types
                | "String" | "bool"
                | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
                | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
                | "f32" | "f64"
                | "Vec" | "Option" | "HashMap" | "BTreeMap"
            )
        } else {
            false
        }
    } else {
        // &str, tuples, references, etc. — not body types
        true
    }
}

/// Rewrites a typed parameter `pat: T` into `Json(pat): Json<T>`.
///
/// This is used by `vil_endpoint` to auto-extract body parameters from
/// JSON request bodies when the type is not a known extractor.
fn rewrite_body_param(pat_type: &syn::PatType) -> FnArg {
    let pat = &pat_type.pat;
    let ty = &pat_type.ty;
    syn::parse_quote! {
        ::vil_server::__private::axum::extract::Json(#pat): ::vil_server::__private::axum::extract::Json<#ty>
    }
}

/// Rewrite function inputs: auto-wrap unknown types with `Json<T>` extraction.
///
/// Parameters whose type is a known extractor or primitive are left as-is.
/// Parameters whose type is an unknown struct/enum are rewritten from
/// `body: CreateOrder` to `Json(body): Json<CreateOrder>`.
fn rewrite_inputs(
    inputs: &syn::punctuated::Punctuated<FnArg, Token![,]>,
) -> syn::punctuated::Punctuated<FnArg, Token![,]> {
    inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Typed(pat_type) => {
                // Only rewrite simple `ident: Type` patterns (not destructuring like `Path(id)`)
                if let Pat::Ident(_) = &*pat_type.pat {
                    if !is_known_extractor(&pat_type.ty) {
                        return rewrite_body_param(pat_type);
                    }
                }
                arg.clone()
            }
            _ => arg.clone(),
        })
        .collect()
}

/// Attribute macro that marks an async function as a VX endpoint.
///
/// Unlike `vil_handler`, this macro does **not** inject `RequestId` as
/// the first parameter. It is a lightweight annotation that:
///
/// 1. Auto-extracts typed body parameters — if a parameter's type is not a
///    known extractor or primitive, it is automatically wrapped with
///    `Json<T>` for JSON body extraction.
/// 2. Adds a `tracing::info_span` around the handler body
/// 3. Optionally dispatches the handler based on an execution class
///
/// # Body Auto-Extraction
///
/// ```ignore
/// // Developer writes:
/// #[vil_endpoint]
/// async fn create_order(body: CreateOrder) -> VilResult<Order> { ... }
///
/// // Macro generates:
/// async fn create_order(
///     Json(body): Json<CreateOrder>,
/// ) -> VilResult<Order> { ... }
/// ```
///
/// Known extractors (`Json`, `Path`, `Query`, `State`, `Extension`, etc.)
/// and primitive types (`u64`, `String`, `bool`, etc.) are left as-is.
///
/// # Execution Classes
///
/// - `AsyncTask` (default) — runs on the Tokio async executor as-is.
/// - `BlockingTask` — wraps the body in `tokio::task::spawn_blocking`.
/// - `DedicatedThread` — wraps the body in `tokio::task::spawn_blocking`.
///
/// # Examples
///
/// ```ignore
/// #[vil_endpoint]
/// async fn get_order(Path(id): Path<u64>) -> VilResult<Order> {
///     // runs as a normal async task; Path is a known extractor, kept as-is
/// }
///
/// #[vil_endpoint]
/// async fn create_order(body: CreateOrder) -> VilResult<Order> {
///     // `body` is auto-wrapped: Json(body): Json<CreateOrder>
/// }
///
/// #[vil_endpoint(exec = BlockingTask)]
/// async fn heavy_compute(body: Json<Input>) -> Json<Output> {
///     // body runs inside spawn_blocking; Json is known, kept as-is
/// }
/// ```
#[proc_macro_attribute]
pub fn vil_endpoint(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as EndpointAttr);
    let input = parse_macro_input!(item as ItemFn);

    let vis = &input.vis;
    let sig = &input.sig;
    let name = &sig.ident;
    let name_str = name.to_string();
    let body = &input.block;
    let asyncness = &sig.asyncness;
    let return_type = &sig.output;
    let attrs_on_fn = &input.attrs;

    // Rewrite inputs: auto-wrap unknown types with Json<T>
    let rewritten_inputs = rewrite_inputs(&sig.inputs);

    let exec = attrs
        .exec_class
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "AsyncTask".to_string());

    match exec.as_str() {
        "AsyncTask" => {
            // Default: keep the function async, wrap body with tracing span.
            // Uses tracing::Instrument instead of Span::entered() so the span
            // is compatible with .await points (Entered<'_> is !Send which
            // breaks async handlers that hold it across await boundaries).
            let expanded = quote! {
                #(#attrs_on_fn)*
                #vis #asyncness fn #name(#rewritten_inputs) #return_type {
                    use ::vil_server::__private::tracing::Instrument as _;
                    async move #body
                        .instrument(::vil_server::__private::tracing::info_span!(
                            "vx_endpoint",
                            endpoint = #name_str,
                            exec_class = "AsyncTask",
                        ))
                        .await
                }
            };
            TokenStream::from(expanded)
        }
        "BlockingTask" | "DedicatedThread" => {
            // Wrap the body in spawn_blocking. The outer function remains
            // async so Axum routing still works. The inner closure is
            // Send + 'static so it must capture parameters by move.
            let inner_name = format_ident!("__vil_vx_inner_{}", name);

            // Collect param names for forwarding — use the ORIGINAL inputs
            // since the rewritten destructuring patterns (e.g. `Json(body)`)
            // still bind the same variable names.
            let param_names: Vec<_> = sig
                .inputs
                .iter()
                .filter_map(|arg| {
                    if let FnArg::Typed(pt) = arg {
                        if let Pat::Ident(pi) = &*pt.pat {
                            Some(pi.ident.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            let exec_class_str = exec.as_str();

            // For the inner function, use original inputs (no Json wrapping)
            // since the outer function already extracted the values.
            let original_inputs = &sig.inputs;

            let expanded = quote! {
                #(#attrs_on_fn)*
                #vis async fn #name(#rewritten_inputs) #return_type {
                    let _span = ::vil_server::__private::tracing::info_span!(
                        "vx_endpoint",
                        endpoint = #name_str,
                        exec_class = #exec_class_str,
                    )
                    .entered();

                    // Inner (sync) function that contains the original body
                    fn #inner_name(#original_inputs) #return_type
                        #body

                    ::tokio::task::spawn_blocking(move || {
                        #inner_name(#(#param_names),*)
                    })
                    .await
                    .expect("spawn_blocking task panicked")
                }
            };
            TokenStream::from(expanded)
        }
        other => {
            let msg = format!(
                "unsupported exec class `{}`. Expected AsyncTask, BlockingTask, or DedicatedThread",
                other
            );
            let err = syn::Error::new_spanned(attrs.exec_class.unwrap(), msg);
            TokenStream::from(err.to_compile_error())
        }
    }
}

// =============================================================================
// vil_app! — Declarative DSL for VX application definition
// =============================================================================

/// A single endpoint entry parsed from the DSL.
struct AppEndpoint {
    method: Ident,
    path: LitStr,
    handler: Ident,
}

/// Parsed content of `vil_app! { ... }`.
struct VilAppDsl {
    name: LitStr,
    port: LitInt,
    endpoints: Vec<AppEndpoint>,
}

impl Parse for VilAppDsl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut port: Option<LitInt> = None;
        let mut endpoints: Vec<AppEndpoint> = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _colon: Token![:] = input.parse()?;

            if key == "name" {
                name = Some(input.parse::<LitStr>()?);
                // optional trailing comma
                if input.peek(Token![,]) {
                    let _: Token![,] = input.parse()?;
                }
            } else if key == "port" {
                port = Some(input.parse::<LitInt>()?);
                if input.peek(Token![,]) {
                    let _: Token![,] = input.parse()?;
                }
            } else if key == "endpoints" {
                let content;
                syn::braced!(content in input);

                while !content.is_empty() {
                    let method: Ident = content.parse()?;
                    let path: LitStr = content.parse()?;
                    let _arrow: Token![=>] = content.parse()?;
                    let handler: Ident = content.parse()?;

                    endpoints.push(AppEndpoint {
                        method,
                        path,
                        handler,
                    });

                    // optional trailing comma
                    if content.peek(Token![,]) {
                        let _: Token![,] = content.parse()?;
                    }
                }

                if input.peek(Token![,]) {
                    let _: Token![,] = input.parse()?;
                }
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "unknown field in vil_app!, expected `name`, `port`, or `endpoints`",
                ));
            }
        }

        let name = name.ok_or_else(|| input.error("vil_app! requires `name: \"...\"`"))?;
        let port = port.ok_or_else(|| input.error("vil_app! requires `port: <number>`"))?;

        Ok(VilAppDsl {
            name,
            port,
            endpoints,
        })
    }
}

/// Declarative macro for defining a VX application.
///
/// Generates a `#[tokio::main] async fn main()` that creates a
/// `ServiceProcess` with all endpoints, wraps it in a `VilApp`, and
/// calls `.run().await`.
///
/// # Syntax
///
/// ```ignore
/// vil_app! {
///     name: "order-service",
///     port: 8080,
///
///     endpoints: {
///         GET  "/"              => hello,
///         POST "/api/orders"    => create_order,
///         GET  "/api/orders/:id" => get_order,
///     }
/// }
/// ```
///
/// # Generated Code
///
/// The macro expands to roughly:
///
/// ```ignore
/// #[tokio::main]
/// async fn main() {
///     let service = ServiceProcess::new("order-service")
///         .endpoint(Method::GET, "/", get(hello))
///         .endpoint(Method::POST, "/api/orders", post(create_order))
///         .endpoint(Method::GET, "/api/orders/:id", get(get_order));
///
///     VilApp::new("order-service")
///         .port(8080)
///         .service(service)
///         .run()
///         .await;
/// }
/// ```
#[proc_macro]
pub fn vil_app(input: TokenStream) -> TokenStream {
    let dsl = parse_macro_input!(input as VilAppDsl);

    let app_name = &dsl.name;
    let port = &dsl.port;

    // Generate endpoint registrations and method routing
    let endpoint_calls: Vec<_> = dsl
        .endpoints
        .iter()
        .map(|ep| {
            let method_str = ep.method.to_string().to_uppercase();
            let path = &ep.path;
            let handler = &ep.handler;

            // Map HTTP method string to axum::http::Method and routing fn
            let (method_path, route_fn) = match method_str.as_str() {
                "GET" => (
                    quote! { ::vil_server::__private::axum::http::Method::GET },
                    quote! { ::vil_server::__private::axum::routing::get(#handler) },
                ),
                "POST" => (
                    quote! { ::vil_server::__private::axum::http::Method::POST },
                    quote! { ::vil_server::__private::axum::routing::post(#handler) },
                ),
                "PUT" => (
                    quote! { ::vil_server::__private::axum::http::Method::PUT },
                    quote! { ::vil_server::__private::axum::routing::put(#handler) },
                ),
                "DELETE" => (
                    quote! { ::vil_server::__private::axum::http::Method::DELETE },
                    quote! { ::vil_server::__private::axum::routing::delete(#handler) },
                ),
                "PATCH" => (
                    quote! { ::vil_server::__private::axum::http::Method::PATCH },
                    quote! { ::vil_server::__private::axum::routing::patch(#handler) },
                ),
                "HEAD" => (
                    quote! { ::vil_server::__private::axum::http::Method::HEAD },
                    quote! { ::vil_server::__private::axum::routing::head(#handler) },
                ),
                "OPTIONS" => (
                    quote! { ::vil_server::__private::axum::http::Method::OPTIONS },
                    quote! { ::vil_server::__private::axum::routing::options(#handler) },
                ),
                _ => {
                    let msg = format!("unsupported HTTP method `{}`", method_str);
                    return quote! { compile_error!(#msg) };
                }
            };

            quote! {
                .endpoint(#method_path, #path, #route_fn)
            }
        })
        .collect();

    let expanded = quote! {
        #[::tokio::main]
        async fn main() {
            let service = ::vil_server::__private::vx::service::ServiceProcess::new(#app_name)
                #(#endpoint_calls)*;

            ::vil_server::__private::vx::app::VilApp::new(#app_name)
                .port(#port)
                .service(service)
                .run()
                .await;
        }
    };

    TokenStream::from(expanded)
}

// =============================================================================
// vil_service_state — Marks a struct as VX managed service state
// =============================================================================

/// Parsed attributes for `#[vil_service_state(storage = ...)]`.
struct ServiceStateAttr {
    storage: String,
}

impl Parse for ServiceStateAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(ServiceStateAttr {
                storage: "PrivateHeap".to_string(),
            });
        }

        let mut storage = "PrivateHeap".to_string();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            if key == "storage" {
                let _eq: Token![=] = input.parse()?;
                let value: Ident = input.parse()?;
                let val_str = value.to_string();
                match val_str.as_str() {
                    "PrivateHeap" | "SharedShm" => {
                        storage = val_str;
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            value,
                            format!(
                                "unsupported storage type `{}`. Expected PrivateHeap or SharedShm",
                                other
                            ),
                        ));
                    }
                }
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "unknown vil_service_state attribute, expected `storage`",
                ));
            }

            // consume optional trailing comma
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        Ok(ServiceStateAttr { storage })
    }
}

/// Attribute macro that marks a struct as VX managed service state.
///
/// Generates a const marker `VIL_SERVICE_STATE: bool = true` and a
/// `VIL_STATE_STORAGE: &'static str` indicating the storage backend.
///
/// # Storage Types
///
/// - `PrivateHeap` (default) — state lives in process-local heap.
/// - `SharedShm` — state is placed in shared memory for cross-process access.
///
/// # Examples
///
/// ```ignore
/// #[vil_service_state]
/// struct OrderState {
///     db: DbPool,
///     next_id: AtomicU64,
/// }
///
/// #[vil_service_state(storage = SharedShm)]
/// struct SharedCounter {
///     count: AtomicU64,
/// }
/// ```
#[proc_macro_attribute]
pub fn vil_service_state(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as ServiceStateAttr);
    let input: proc_macro2::TokenStream = item.into();

    // Parse the struct to get its name
    let item_struct: syn::ItemStruct = match syn::parse2(input.clone()) {
        Ok(s) => s,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let name = &item_struct.ident;
    let storage = &attrs.storage;

    let expanded = quote! {
        #item_struct

        impl #name {
            /// Marker: this type is managed by VX as service state.
            /// VflowHost can call init/shutdown on it during provisioning lifecycle.
            pub const VIL_SERVICE_STATE: bool = true;

            /// Storage backend for this service state.
            pub const VIL_STATE_STORAGE: &'static str = #storage;
        }
    };

    TokenStream::from(expanded)
}

// =============================================================================
// vil_service — Module-level attribute macro for VX service definition
// =============================================================================

/// Parsed attributes for `#[vil_service(name = "...", prefix = "...", requires = [...])]`.
struct ServiceAttr {
    name: LitStr,
    prefix: Option<LitStr>,
    requires: Vec<LitStr>,
}

impl Parse for ServiceAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut prefix: Option<LitStr> = None;
        let mut requires: Vec<LitStr> = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;

            if key == "name" {
                let _eq: Token![=] = input.parse()?;
                name = Some(input.parse::<LitStr>()?);
            } else if key == "prefix" {
                let _eq: Token![=] = input.parse()?;
                prefix = Some(input.parse::<LitStr>()?);
            } else if key == "requires" {
                let _eq: Token![=] = input.parse()?;
                let content;
                syn::bracketed!(content in input);
                while !content.is_empty() {
                    requires.push(content.parse::<LitStr>()?);
                    if content.peek(Token![,]) {
                        let _: Token![,] = content.parse()?;
                    }
                }
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "unknown vil_service attribute, expected `name`, `prefix`, or `requires`",
                ));
            }

            // consume optional trailing comma
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let name = name.ok_or_else(|| input.error("vil_service requires `name = \"...\"`"))?;

        Ok(ServiceAttr {
            name,
            prefix,
            requires,
        })
    }
}

/// Attribute macro that wraps a module as a VX service definition.
///
/// Generates a `service()` factory function that returns a configured
/// `ServiceProcess`, along with constants for the service name, prefix,
/// and mesh requirements.
///
/// # Parameters
///
/// - `name` (required) — service name, must be unique within the application.
/// - `prefix` (optional) — URL prefix; defaults to `/api/{name}`.
/// - `requires` (optional) — list of mesh dependencies as `"service:Lane"` strings.
///
/// # Examples
///
/// ```ignore
/// #[vil_service(name = "orders", prefix = "/api")]
/// mod orders {
///     use super::*;
///
///     #[vil_endpoint]
///     async fn list() -> &'static str { "[]" }
/// }
///
/// #[vil_service(name = "payments", prefix = "/pay", requires = ["auth:Trigger"])]
/// mod payments {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn vil_service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as ServiceAttr);
    let mut module: ItemMod = match syn::parse(item) {
        Ok(m) => m,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let service_name = &attrs.name;
    let service_name_val = attrs.name.value();

    // Determine prefix: explicit or default /api/{name}
    let prefix_str = match &attrs.prefix {
        Some(lit) => lit.value(),
        None => format!("/api/{}", service_name_val),
    };
    let prefix_lit = LitStr::new(&prefix_str, attrs.name.span());

    // Generate the prefix builder call
    let prefix_call = quote! { .prefix(#prefix_lit) };

    // Generate MESH_REQUIRES const
    let requires_items = &attrs.requires;
    let mesh_requires = quote! {
        /// Mesh dependencies declared via `requires = [...]`.
        pub const MESH_REQUIRES: &[&str] = &[#(#requires_items),*];
    };

    // Build the generated items to inject into the module
    let generated = quote! {
        /// Auto-generated service factory function.
        /// Returns a configured ServiceProcess ready to be added to VilApp.
        pub fn service() -> ::vil_server::__private::vx::service::ServiceProcess {
            ::vil_server::__private::vx::service::ServiceProcess::new(#service_name)
                #prefix_call
        }

        /// Service name constant.
        pub const SERVICE_NAME: &str = #service_name;

        /// Service prefix constant.
        pub const SERVICE_PREFIX: &str = #prefix_lit;

        #mesh_requires
    };

    // Inject the generated items into the module body
    if let Some((brace, ref mut items)) = module.content {
        let generated_items: syn::File =
            syn::parse2(generated).expect("generated code should parse");
        items.extend(generated_items.items);
        module.content = Some((brace, items.clone()));
    } else {
        // Module with no body (e.g. `mod foo;`) — error
        return TokenStream::from(
            syn::Error::new_spanned(module, "vil_service requires an inline module body")
                .to_compile_error(),
        );
    }

    TokenStream::from(quote! { #module })
}
