// =============================================================================
// VIL DB Macros — Derive macros for VilEntity
// =============================================================================
//
// #[derive(VilEntity)] generates:
//   - VilEntityMeta impl with const TABLE, SOURCE, PRIMARY_KEY, FIELDS
//   - Zero runtime cost — all values are compile-time constants

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Lit, Meta, NestedMeta};

/// Derive macro for VilEntity.
///
/// Generates VilEntityMeta impl with const metadata.
///
/// # Attributes
/// - `#[vil(source = "datasource_name")]` — datasource alias
/// - `#[vil(table = "table_name")]` — table name (default: struct name lowercase)
/// - `#[vil(primary_key)]` — mark primary key field
///
/// # Example
/// ```ignore
/// #[derive(VilEntity)]
/// #[vil(source = "main_db", table = "orders")]
/// struct Order {
///     #[vil(primary_key)]
///     id: i64,
///     customer_id: i64,
///     total: f64,
/// }
/// ```
#[proc_macro_derive(VilEntity, attributes(vil))]
pub fn derive_vil_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse struct-level attributes
    let mut source = String::from("default");
    let mut table = name.to_string().to_lowercase();

    for attr in &input.attrs {
        if attr.path.is_ident("vil") {
            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                for nested in &meta_list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(nv)) = nested {
                        if nv.path.is_ident("source") {
                            if let Lit::Str(lit) = &nv.lit {
                                source = lit.value();
                            }
                        }
                        if nv.path.is_ident("table") {
                            if let Lit::Str(lit) = &nv.lit {
                                table = lit.value();
                            }
                        }
                    }
                }
            }
        }
    }

    // Parse field-level attributes
    let mut primary_key = String::from("id");
    let mut fields = Vec::new();

    if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(named_fields) = &data_struct.fields {
            for field in &named_fields.named {
                if let Some(ident) = &field.ident {
                    fields.push(ident.to_string());

                    // Check for #[vil(primary_key)]
                    for attr in &field.attrs {
                        if attr.path.is_ident("vil") {
                            if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                                for nested in &meta_list.nested {
                                    if let NestedMeta::Meta(Meta::Path(path)) = nested {
                                        if path.is_ident("primary_key") {
                                            primary_key = ident.to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let field_strs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
    let _field_count = field_strs.len();

    let expanded = quote! {
        impl vil_db_semantic::VilEntityMeta for #name {
            const TABLE: &'static str = #table;
            const SOURCE: &'static str = #source;
            const PRIMARY_KEY: &'static str = #primary_key;
            const FIELDS: &'static [&'static str] = &[#(#field_strs),*];
        }
    };

    TokenStream::from(expanded)
}
