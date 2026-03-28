// =============================================================================
// VilError derive — generates Display, Error, From<T> for VilError
// =============================================================================

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// Parsed attributes from `#[vil_error(status = NNN, code = "...", retry = bool)]`.
struct ErrorAttr {
    status: u16,
    code: Option<String>,
    retry: Option<bool>,
}

/// Parse `#[vil_error(status = NNN, code = "...", retry = bool)]` from a variant's attributes.
fn parse_error_attr(attrs: &[syn::Attribute]) -> ErrorAttr {
    let mut result = ErrorAttr {
        status: 500,
        code: None,
        retry: None,
    };
    for attr in attrs {
        if attr.path().is_ident("vil_error") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("status") {
                    let value = meta.value()?;
                    let lit: syn::LitInt = value.parse()?;
                    result.status = lit.base10_parse::<u16>()?;
                } else if meta.path.is_ident("code") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    result.code = Some(lit.value());
                } else if meta.path.is_ident("retry") {
                    let value = meta.value()?;
                    let lit: syn::LitBool = value.parse()?;
                    result.retry = Some(lit.value());
                }
                Ok(())
            });
        }
    }
    result
}

/// Map a status code to the VilError factory method name (as an ident string).
fn status_to_factory_call(
    status: u16,
    detail_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match status {
        400 => quote! { ::vil_server_core::VilError::bad_request(#detail_expr) },
        401 => quote! { ::vil_server_core::VilError::unauthorized(#detail_expr) },
        403 => quote! { ::vil_server_core::VilError::forbidden(#detail_expr) },
        404 => quote! { ::vil_server_core::VilError::not_found(#detail_expr) },
        422 => quote! { ::vil_server_core::VilError::validation(#detail_expr) },
        429 => quote! { ::vil_server_core::VilError::rate_limited() },
        500 => quote! { ::vil_server_core::VilError::internal(#detail_expr) },
        503 => quote! { ::vil_server_core::VilError::service_unavailable(#detail_expr) },
        _ => quote! { ::vil_server_core::VilError::internal(#detail_expr) },
    }
}

/// Convert a PascalCase variant name to SCREAMING_SNAKE_CASE.
fn to_screaming_snake(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_uppercase());
    }
    result
}

pub fn derive_vil_error_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => {
            return syn::Error::new_spanned(name, "VilError can only be derived for enums")
                .to_compile_error()
                .into();
        }
    };

    // Collect info for each variant
    let mut display_arms = Vec::new();
    let mut from_arms = Vec::new();
    let mut const_items = Vec::new();

    for variant in variants {
        let var_ident = &variant.ident;
        let var_name = var_ident.to_string();
        let error_attr = parse_error_attr(&variant.attrs);
        let screaming = to_screaming_snake(&var_name);

        // Generate constants for code and retry if present
        if let Some(ref code) = error_attr.code {
            let code_const = syn::Ident::new(
                &format!("{}_CODE", screaming),
                proc_macro2::Span::call_site(),
            );
            const_items.push(quote! {
                pub const #code_const: &'static str = #code;
            });
        }

        {
            let retry_const = syn::Ident::new(
                &format!("{}_RETRY", screaming),
                proc_macro2::Span::call_site(),
            );
            let retry_val = error_attr.retry.unwrap_or(false);
            const_items.push(quote! {
                pub const #retry_const: bool = #retry_val;
            });
        }

        // Build the detail expression for the From impl
        let detail_expr = if let Some(ref code) = error_attr.code {
            quote! { format!("[{}] {}", #code, e) }
        } else {
            quote! { e.to_string() }
        };

        let factory = status_to_factory_call(error_attr.status, detail_expr);

        match &variant.fields {
            Fields::Named(fields_named) => {
                // e.g. NotFound { id: u64 }
                let field_idents: Vec<_> = fields_named
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let field_names: Vec<String> = field_idents.iter().map(|i| i.to_string()).collect();

                // Build format string: "NotFound: id={}, other={}"
                let fmt_parts: Vec<String> =
                    field_names.iter().map(|n| format!("{}={{}}", n)).collect();
                let fmt_str = format!("{}: {}", var_name, fmt_parts.join(", "));

                display_arms.push(quote! {
                    #name::#var_ident { #(#field_idents),* } => write!(f, #fmt_str, #(#field_idents),*)
                });

                from_arms.push(quote! {
                    e @ #name::#var_ident { .. } => #factory
                });
            }
            Fields::Unnamed(fields_unnamed) => {
                // e.g. DatabaseError(String) — one or more positional fields
                let count = fields_unnamed.unnamed.len();
                if count == 1 {
                    display_arms.push(quote! {
                        #name::#var_ident(ref msg) => write!(f, "{}: {}", #var_name, msg)
                    });
                } else {
                    // Multiple positional fields: _0, _1, ...
                    let bindings: Vec<syn::Ident> = (0..count)
                        .map(|i| {
                            syn::Ident::new(&format!("_{}", i), proc_macro2::Span::call_site())
                        })
                        .collect();
                    let fmt_placeholders: String = (0..count)
                        .map(|_| "{}".to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let fmt_str = format!("{}: {}", var_name, fmt_placeholders);
                    display_arms.push(quote! {
                        #name::#var_ident(#(ref #bindings),*) => write!(f, #fmt_str, #(#bindings),*)
                    });
                }

                from_arms.push(quote! {
                    e @ #name::#var_ident(..) => #factory
                });
            }
            Fields::Unit => {
                // e.g. InvalidTitle
                display_arms.push(quote! {
                    #name::#var_ident => write!(f, "{}", #var_name)
                });

                from_arms.push(quote! {
                    e @ #name::#var_ident => #factory
                });
            }
        }
    }

    let expanded = quote! {
        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #(#display_arms),*
                }
            }
        }

        impl ::std::error::Error for #name {}

        impl ::core::convert::From<#name> for ::vil_server_core::VilError {
            fn from(e: #name) -> Self {
                match e {
                    #(#from_arms),*
                }
            }
        }

        impl #name {
            #(#const_items)*
        }
    };

    TokenStream::from(expanded)
}
