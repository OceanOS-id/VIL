//! # vil_connector_macros
//!
//! Lightweight proc-macro crate for VIL connector crates.
//!
//! Provides three attribute macros:
//! - `#[connector_fault]`  — applied to enums, generates error_code/kind/is_retryable/Display/Error
//! - `#[connector_event]`  — applied to structs, generates repr(C)/Default/size guard (≤192 bytes)
//! - `#[connector_state]`  — applied to structs, generates repr(C)/Default (no size guard)
//!
//! Zero VIL runtime dependencies — only `syn`, `quote`, `proc-macro2`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

// =============================================================================
// Helper: detect retryable variants by name
// =============================================================================

/// Returns true if the variant name contains any retryable keyword (case-insensitive).
fn is_retryable_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("timeout")
        || lower.contains("connectionfailed")
        || lower.contains("unavailable")
        || lower.contains("retry")
}

// =============================================================================
// #[connector_fault] — enum
// =============================================================================

/// Attribute macro for connector fault enums.
///
/// Adds `#[derive(Debug, Clone, Copy)]` and generates:
/// - `error_code() -> u32` — variant index starting from 1
/// - `kind() -> &'static str` — variant name as string
/// - `is_retryable() -> bool` — true for Timeout/ConnectionFailed/Unavailable/Retry variants
/// - `impl std::fmt::Display`
/// - `impl std::error::Error`
///
/// # Example
/// ```ignore
/// #[connector_fault]
/// pub enum MongoFault {
///     ConnectionFailed { uri_hash: u32, reason_code: u32 },
///     QueryFailed { collection_hash: u32, reason_code: u32 },
///     Timeout { collection_hash: u32, elapsed_ms: u32 },
/// }
/// ```
#[proc_macro_attribute]
pub fn connector_fault(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match expand_connector_fault(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_connector_fault(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;

    // Must be an enum
    let data_enum = match &input.data {
        Data::Enum(e) => e,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "#[connector_fault] can only be applied to enums",
            ))
        }
    };

    let variants: Vec<_> = data_enum.variants.iter().collect();

    // Build match arms for error_code
    let error_code_arms = variants.iter().enumerate().map(|(i, v)| {
        let vname = &v.ident;
        let code = (i + 1) as u32;
        match &v.fields {
            Fields::Unit => quote! { Self::#vname => #code, },
            Fields::Named(_) => quote! { Self::#vname { .. } => #code, },
            Fields::Unnamed(_) => quote! { Self::#vname(..) => #code, },
        }
    });

    // Build match arms for kind
    let kind_arms = variants.iter().map(|v| {
        let vname = &v.ident;
        let vname_str = vname.to_string();
        match &v.fields {
            Fields::Unit => quote! { Self::#vname => #vname_str, },
            Fields::Named(_) => quote! { Self::#vname { .. } => #vname_str, },
            Fields::Unnamed(_) => quote! { Self::#vname(..) => #vname_str, },
        }
    });

    // Build match arms for is_retryable
    let retryable_arms = variants.iter().map(|v| {
        let vname = &v.ident;
        let retryable = is_retryable_name(&vname.to_string());
        let retryable_lit = if retryable {
            quote! { true }
        } else {
            quote! { false }
        };
        match &v.fields {
            Fields::Unit => quote! { Self::#vname => #retryable_lit, },
            Fields::Named(_) => quote! { Self::#vname { .. } => #retryable_lit, },
            Fields::Unnamed(_) => quote! { Self::#vname(..) => #retryable_lit, },
        }
    });

    // Reconstruct the enum variants for emission
    let enum_variants = variants.iter().map(|v| {
        let vattrs = &v.attrs;
        let vname = &v.ident;
        let fields = &v.fields;
        match fields {
            Fields::Unit => quote! { #(#vattrs)* #vname },
            Fields::Named(named) => {
                let fs = &named.named;
                quote! { #(#vattrs)* #vname { #fs } }
            }
            Fields::Unnamed(unnamed) => {
                let fs = &unnamed.unnamed;
                quote! { #(#vattrs)* #vname(#fs) }
            }
        }
    });

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        #(#attrs)*
        #[derive(Debug, Clone, Copy)]
        #vis enum #name #generics {
            #(#enum_variants,)*
        }

        impl #impl_generics #name #ty_generics #where_clause {
            /// Numeric error code (variant index starting from 1).
            pub fn error_code(&self) -> u32 {
                match self {
                    #(#error_code_arms)*
                }
            }

            /// Variant name as string.
            pub fn kind(&self) -> &'static str {
                match self {
                    #(#kind_arms)*
                }
            }

            /// Whether this fault is retryable.
            /// Convention: variants named "Timeout", "ConnectionFailed", "Unavailable", or "Retry" are retryable.
            pub fn is_retryable(&self) -> bool {
                match self {
                    #(#retryable_arms)*
                }
            }
        }

        impl #impl_generics std::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}(code={})", self.kind(), self.error_code())
            }
        }

        impl #impl_generics std::error::Error for #name #ty_generics #where_clause {}
    })
}

// =============================================================================
// #[connector_event] — struct with 192-byte size guard
// =============================================================================

/// Attribute macro for connector event structs.
///
/// Adds `#[derive(Debug, Clone, Copy)]`, `#[repr(C)]`, a zeroed `Default` impl,
/// and a compile-time assertion that the struct fits within 192 bytes (LogSlot payload).
///
/// # Example
/// ```ignore
/// #[connector_event]
/// pub struct MongoDocumentInserted {
///     pub collection_hash: u32,
///     pub document_id_hash: u32,
///     pub size_bytes: u32,
///     pub timestamp_ns: u64,
/// }
/// ```
#[proc_macro_attribute]
pub fn connector_event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match expand_connector_event(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_connector_event(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;

    // Must be a struct
    let data_struct = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "#[connector_event] can only be applied to structs",
            ))
        }
    };

    let fields = &data_struct.fields;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Reconstruct fields block
    let fields_tokens = match fields {
        Fields::Named(named) => {
            let fs = &named.named;
            quote! { { #fs } }
        }
        Fields::Unnamed(unnamed) => {
            let fs = &unnamed.unnamed;
            quote! { ( #fs ); }
        }
        Fields::Unit => quote! { ; },
    };

    Ok(quote! {
        #(#attrs)*
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        #vis struct #name #generics #fields_tokens

        impl #impl_generics Default for #name #ty_generics #where_clause {
            fn default() -> Self {
                // SAFETY: all-zero bytes are valid for a #[repr(C)] POD event struct
                unsafe { std::mem::zeroed() }
            }
        }

        // Compile-time size guard: connector events must fit within 192 bytes (LogSlot payload)
        const _: () = {
            assert!(
                std::mem::size_of::<#name>() <= 192,
                "connector event must fit within 192 bytes (LogSlot payload)"
            );
        };
    })
}

// =============================================================================
// #[connector_state] — struct, no size guard
// =============================================================================

/// Attribute macro for connector state structs.
///
/// Adds `#[derive(Debug, Clone, Copy)]`, `#[repr(C)]`, and a zeroed `Default` impl.
/// No size constraint — state structs may be larger than 192 bytes.
///
/// # Example
/// ```ignore
/// #[connector_state]
/// pub struct MongoPoolState {
///     pub active_connections: u32,
///     pub idle_connections: u32,
///     pub total_queries: u64,
/// }
/// ```
#[proc_macro_attribute]
pub fn connector_state(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match expand_connector_state(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_connector_state(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;

    // Must be a struct
    let data_struct = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "#[connector_state] can only be applied to structs",
            ))
        }
    };

    let fields = &data_struct.fields;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Reconstruct fields block
    let fields_tokens = match fields {
        Fields::Named(named) => {
            let fs = &named.named;
            quote! { { #fs } }
        }
        Fields::Unnamed(unnamed) => {
            let fs = &unnamed.unnamed;
            quote! { ( #fs ); }
        }
        Fields::Unit => quote! { ; },
    };

    Ok(quote! {
        #(#attrs)*
        #[derive(Debug, Clone, Copy)]
        #[repr(C)]
        #vis struct #name #generics #fields_tokens

        impl #impl_generics Default for #name #ty_generics #where_clause {
            fn default() -> Self {
                // SAFETY: all-zero bytes are valid for a #[repr(C)] POD state struct
                unsafe { std::mem::zeroed() }
            }
        }
    })
}

// =============================================================================
// Unit tests for helper functions (no proc-macro machinery needed)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::is_retryable_name;

    #[test]
    fn retryable_timeout() {
        assert!(is_retryable_name("Timeout"));
        assert!(is_retryable_name("RequestTimeout"));
        assert!(is_retryable_name("TIMEOUT"));
    }

    #[test]
    fn retryable_connection_failed() {
        assert!(is_retryable_name("ConnectionFailed"));
        assert!(is_retryable_name("connectionfailed"));
    }

    #[test]
    fn retryable_unavailable() {
        assert!(is_retryable_name("ServiceUnavailable"));
        assert!(is_retryable_name("Unavailable"));
    }

    #[test]
    fn retryable_retry() {
        assert!(is_retryable_name("RetryExhausted"));
        assert!(is_retryable_name("ShouldRetry"));
    }

    #[test]
    fn not_retryable() {
        assert!(!is_retryable_name("QueryFailed"));
        assert!(!is_retryable_name("ParseError"));
        assert!(!is_retryable_name("InvalidInput"));
        assert!(!is_retryable_name("AuthFailed"));
    }
}
