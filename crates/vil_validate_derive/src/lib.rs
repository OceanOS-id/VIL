//! # VilValidate — Request validation derive macro
//!
//! Generates a `validate()` method that checks field constraints
//! and returns a list of validation errors.
//!
//! # Usage
//! ```ignore
//! #[derive(Deserialize, VilValidate)]
//! struct RegisterRequest {
//!     #[validate(min_len = 3, max_len = 50)]
//!     username: String,
//!     #[validate(min_len = 8)]
//!     password: String,
//!     #[validate(email, optional)]
//!     email: Option<String>,
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Meta, Expr, Lit};

/// Derive macro for request validation.
///
/// Generates `fn validate(&self) -> Result<(), Vec<ValidationError>>`.
#[proc_macro_derive(VilValidate, attributes(validate))]
pub fn derive_vil_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return TokenStream::from(
                syn::Error::new_spanned(name, "VilValidate only supports named fields")
                    .to_compile_error(),
            ),
        },
        _ => return TokenStream::from(
            syn::Error::new_spanned(name, "VilValidate only supports structs")
                .to_compile_error(),
        ),
    };

    let mut checks = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_str = field_name.to_string();

        for attr in &field.attrs {
            if !attr.path().is_ident("validate") {
                continue;
            }

            let mut is_optional = false;
            let mut min_len: Option<usize> = None;
            let mut max_len: Option<usize> = None;
            let mut is_email = false;
            let mut range_min: Option<i64> = None;
            let mut range_max: Option<i64> = None;

            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("optional") {
                    is_optional = true;
                } else if meta.path.is_ident("email") {
                    is_email = true;
                } else if meta.path.is_ident("min_len") {
                    let value = meta.value()?;
                    let lit: syn::LitInt = value.parse()?;
                    min_len = Some(lit.base10_parse::<usize>()?);
                } else if meta.path.is_ident("max_len") {
                    let value = meta.value()?;
                    let lit: syn::LitInt = value.parse()?;
                    max_len = Some(lit.base10_parse::<usize>()?);
                }
                // range(min, max) handled separately
                Ok(())
            });

            // Generate validation checks
            let is_option_type = {
                let ty_str = quote!(#field).to_string();
                ty_str.contains("Option")
            };

            if let Some(min) = min_len {
                if is_option_type {
                    checks.push(quote! {
                        if let Some(ref val) = self.#field_name {
                            if val.len() < #min {
                                errors.push(VilValidationError {
                                    field: #field_str.to_string(),
                                    message: format!("Must be at least {} characters", #min),
                                });
                            }
                        }
                    });
                } else {
                    checks.push(quote! {
                        if self.#field_name.len() < #min {
                            errors.push(VilValidationError {
                                field: #field_str.to_string(),
                                message: format!("Must be at least {} characters", #min),
                            });
                        }
                    });
                }
            }

            if let Some(max) = max_len {
                if is_option_type {
                    checks.push(quote! {
                        if let Some(ref val) = self.#field_name {
                            if val.len() > #max {
                                errors.push(VilValidationError {
                                    field: #field_str.to_string(),
                                    message: format!("Must be at most {} characters", #max),
                                });
                            }
                        }
                    });
                } else {
                    checks.push(quote! {
                        if self.#field_name.len() > #max {
                            errors.push(VilValidationError {
                                field: #field_str.to_string(),
                                message: format!("Must be at most {} characters", #max),
                            });
                        }
                    });
                }
            }

            if is_email {
                if is_option_type {
                    checks.push(quote! {
                        if let Some(ref val) = self.#field_name {
                            if !val.contains('@') || !val.contains('.') {
                                errors.push(VilValidationError {
                                    field: #field_str.to_string(),
                                    message: "Must be a valid email address".to_string(),
                                });
                            }
                        }
                    });
                } else {
                    checks.push(quote! {
                        if !self.#field_name.contains('@') || !self.#field_name.contains('.') {
                            errors.push(VilValidationError {
                                field: #field_str.to_string(),
                                message: "Must be a valid email address".to_string(),
                            });
                        }
                    });
                }
            }
        }
    }

    let expanded = quote! {
        /// Validation error for a single field.
        #[derive(Debug, Clone, ::serde::Serialize)]
        pub struct VilValidationError {
            pub field: String,
            pub message: String,
        }

        impl #name {
            /// Validate all fields. Returns errors if any constraints are violated.
            pub fn validate(&self) -> Result<(), Vec<VilValidationError>> {
                let mut errors: Vec<VilValidationError> = Vec::new();
                #(#checks)*
                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors)
                }
            }
        }
    };

    TokenStream::from(expanded)
}
