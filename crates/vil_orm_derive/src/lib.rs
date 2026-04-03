//! # VilORM Derive — `#[derive(VilEntity)]`
//!
//! Generates find/insert/update/delete methods + query builder + Create/Update structs.
//!
//! ```ignore
//! #[derive(VilEntity)]
//! #[vil_entity(table = "profiles")]
//! pub struct Profile {
//!     #[vil_entity(pk, auto_uuid)]
//!     pub id: String,
//!     #[vil_entity(unique)]
//!     pub username: String,
//!     pub xp: i64,
//!     #[vil_entity(auto_now_add)]
//!     pub created_at: String,
//! }
//! ```

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(VilEntity, attributes(vil_entity))]
pub fn derive_vil_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(f) => &f.named,
            _ => return err(name, "VilEntity requires named fields"),
        },
        _ => return err(name, "VilEntity only supports structs"),
    };

    // Parse struct-level attributes
    let mut table_name = name.to_string().to_lowercase() + "s";
    for attr in &input.attrs {
        if attr.path().is_ident("vil_entity") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("table") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    table_name = lit.value();
                }
                Ok(())
            });
        }
    }

    // Parse field-level attributes
    let mut pk_field: Option<syn::Ident> = None;
    let mut has_auto_uuid = false;
    let mut auto_now_add_fields: Vec<syn::Ident> = Vec::new();
    let mut auto_now_fields: Vec<syn::Ident> = Vec::new();
    let mut unique_fields: Vec<syn::Ident> = Vec::new();
    let mut write_only_fields: Vec<syn::Ident> = Vec::new();
    let mut all_fields: Vec<(syn::Ident, syn::Type)> = Vec::new();

    for field in fields {
        let fname = field.ident.as_ref().unwrap().clone();
        let ftype = field.ty.clone();
        all_fields.push((fname.clone(), ftype));

        for attr in &field.attrs {
            if attr.path().is_ident("vil_entity") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("pk") { pk_field = Some(fname.clone()); }
                    if meta.path.is_ident("auto_uuid") { has_auto_uuid = true; }
                    if meta.path.is_ident("auto_now_add") { auto_now_add_fields.push(fname.clone()); }
                    if meta.path.is_ident("auto_now") { auto_now_fields.push(fname.clone()); }
                    if meta.path.is_ident("unique") { unique_fields.push(fname.clone()); }
                    if meta.path.is_ident("write_only") { write_only_fields.push(fname.clone()); }
                    Ok(())
                });
            }
        }
    }

    let pk = pk_field.unwrap_or_else(|| format_ident!("id"));
    let table = &table_name;

    // Generate field names for SELECT (exclude write_only)
    let select_fields: Vec<_> = all_fields.iter()
        .filter(|(f, _)| !write_only_fields.iter().any(|w| w == f))
        .map(|(f, _)| f.to_string())
        .collect();
    let _select_cols = select_fields.join(", ");

    // Generate field names for INSERT
    let insert_fields: Vec<_> = all_fields.iter()
        .filter(|(f, _)| {
            !auto_now_add_fields.iter().any(|a| a == f) &&
            !auto_now_fields.iter().any(|a| a == f)
        })
        .map(|(f, _)| f.to_string())
        .collect();
    let insert_placeholders: Vec<String> = insert_fields.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect();
    let _insert_cols = insert_fields.join(", ");
    let _insert_vals = insert_placeholders.join(", ");

    // All column names for listing
    let all_col_names: Vec<_> = all_fields.iter().map(|(f, _)| f.to_string()).collect();

    let expanded = quote! {
        impl #name {
            /// Table name
            pub const TABLE: &'static str = #table;
            /// Primary key column
            pub const PK: &'static str = stringify!(#pk);

            /// Fluent query builder (JOOQ-style).
            /// Example: `Profile::q().select(&["id","xp"]).where_eq("id", &uid).fetch_one(pool).await?`
            pub fn q() -> ::vil_orm::VilQuery {
                ::vil_orm::VilQuery::new(#table)
            }

            /// Find by primary key (all columns).
            pub async fn find_by_id(
                pool: &::sqlx::Pool<::sqlx::Any>,
                id: &str,
            ) -> Result<Option<Self>, ::sqlx::Error> {
                let sql = format!("SELECT * FROM {} WHERE {} = $1", #table, stringify!(#pk));
                let start = ::std::time::Instant::now();
                let result = ::sqlx::query_as::<_, Self>(&sql)
                    .bind(id)
                    .fetch_optional(pool)
                    .await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, if result.as_ref().ok().and_then(|r| r.as_ref()).is_some() { 1 } else { 0 }, 0, result.is_err());
                result
            }

            /// Find all (with optional limit, all columns).
            pub async fn find_all(
                pool: &::sqlx::Pool<::sqlx::Any>,
            ) -> Result<Vec<Self>, ::sqlx::Error> {
                let sql = format!("SELECT * FROM {} ORDER BY {} DESC LIMIT 100", #table, stringify!(#pk));
                let start = ::std::time::Instant::now();
                let result = ::sqlx::query_as::<_, Self>(&sql)
                    .fetch_all(pool)
                    .await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, result.as_ref().map(|r| r.len() as u32).unwrap_or(0), 0, result.is_err());
                result
            }

            /// Select specific columns by primary key → custom target type.
            ///
            /// ```ignore
            /// #[derive(sqlx::FromRow)]
            /// struct ProfileSlim { id: String, username: Option<String>, xp: i64 }
            ///
            /// let slim = Profile::select::<ProfileSlim>(pool, &["id","username","xp"], "id = ?", &["abc"]).await?;
            /// ```
            pub async fn select<T: for<'r> ::sqlx::FromRow<'r, ::sqlx::any::AnyRow> + Send + Unpin>(
                pool: &::sqlx::Pool<::sqlx::Any>,
                columns: &[&str],
                condition: &str,
                binds: &[&str],
            ) -> Result<Vec<T>, ::sqlx::Error> {
                let cols = columns.join(", ");
                let sql = format!("SELECT {} FROM {} WHERE {}", cols, #table, condition);
                let mut q = ::sqlx::query_as::<_, T>(&sql);
                for b in binds { q = q.bind(*b); }
                let start = ::std::time::Instant::now();
                let result = q.fetch_all(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, result.as_ref().map(|r| r.len() as u32).unwrap_or(0), 0, result.is_err());
                result
            }

            /// Select specific columns, return one optional row.
            pub async fn select_one<T: for<'r> ::sqlx::FromRow<'r, ::sqlx::any::AnyRow> + Send + Unpin>(
                pool: &::sqlx::Pool<::sqlx::Any>,
                columns: &[&str],
                condition: &str,
                binds: &[&str],
            ) -> Result<Option<T>, ::sqlx::Error> {
                let cols = columns.join(", ");
                let sql = format!("SELECT {} FROM {} WHERE {}", cols, #table, condition);
                let mut q = ::sqlx::query_as::<_, T>(&sql);
                for b in binds { q = q.bind(*b); }
                let start = ::std::time::Instant::now();
                let result = q.fetch_optional(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, if result.as_ref().ok().and_then(|r| r.as_ref()).is_some() { 1 } else { 0 }, 0, result.is_err());
                result
            }

            /// Count all rows
            pub async fn count(
                pool: &::sqlx::Pool<::sqlx::Any>,
            ) -> Result<i64, ::sqlx::Error> {
                let sql = format!("SELECT CAST(COUNT(*) AS INTEGER) FROM {}", #table);
                let start = ::std::time::Instant::now();
                let result = ::sqlx::query_scalar::<_, i64>(&sql)
                    .fetch_one(pool)
                    .await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, 1, 0, result.is_err());
                result
            }

            /// Check if exists by primary key
            pub async fn exists(
                pool: &::sqlx::Pool<::sqlx::Any>,
                id: &str,
            ) -> Result<bool, ::sqlx::Error> {
                let sql = format!("SELECT COUNT(*) FROM {} WHERE {} = $1", #table, stringify!(#pk));
                let start = ::std::time::Instant::now();
                let result = ::sqlx::query_scalar(&sql)
                    .bind(id)
                    .fetch_one(pool)
                    .await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, 1, 0, result.is_err());
                result.map(|count: i64| count > 0)
            }

            /// Delete by primary key. Returns true if row existed.
            pub async fn delete(
                pool: &::sqlx::Pool<::sqlx::Any>,
                id: &str,
            ) -> Result<bool, ::sqlx::Error> {
                let sql = format!("DELETE FROM {} WHERE {} = $1", #table, stringify!(#pk));
                let start = ::std::time::Instant::now();
                let result = ::sqlx::query(&sql)
                    .bind(id)
                    .execute(pool)
                    .await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, result.as_ref().map(|r| if r.rows_affected() > 0 { 1 } else { 0 }).unwrap_or(0), 3, result.is_err());
                result.map(|r| r.rows_affected() > 0)
            }

            /// Column names (for query building)
            pub fn columns() -> &'static [&'static str] {
                &[#(#all_col_names),*]
            }

            /// Find one row matching a WHERE clause (all columns).
            /// For production, prefer `select_one()` with explicit columns.
            /// Example: `Profile::find_where(pool, "username = ?", &["alice"]).await?`
            pub async fn find_where(
                pool: &::sqlx::Pool<::sqlx::Any>,
                condition: &str,
                binds: &[&str],
            ) -> Result<Option<Self>, ::sqlx::Error> {
                let sql = format!("SELECT * FROM {} WHERE {}", #table, condition);
                let mut q = ::sqlx::query_as::<_, Self>(&sql);
                for b in binds { q = q.bind(*b); }
                let start = ::std::time::Instant::now();
                let result = q.fetch_optional(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, if result.as_ref().ok().and_then(|r| r.as_ref()).is_some() { 1 } else { 0 }, 0, result.is_err());
                result
            }

            /// Find all rows matching a WHERE clause (all columns).
            /// For production, prefer `select()` with explicit columns.
            /// Example: `QuizResult::find_all_where(pool, "user_id = ? ORDER BY date DESC LIMIT 100", &[user_id]).await?`
            pub async fn find_all_where(
                pool: &::sqlx::Pool<::sqlx::Any>,
                condition: &str,
                binds: &[&str],
            ) -> Result<Vec<Self>, ::sqlx::Error> {
                let sql = format!("SELECT * FROM {} WHERE {}", #table, condition);
                let mut q = ::sqlx::query_as::<_, Self>(&sql);
                for b in binds { q = q.bind(*b); }
                let start = ::std::time::Instant::now();
                let result = q.fetch_all(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, result.as_ref().map(|r| r.len() as u32).unwrap_or(0), 0, result.is_err());
                result
            }

            /// Update specific fields by primary key.
            /// Example: `Profile::update_fields(pool, id, &[("xp", "100"), ("bio", "hello")]).await?`
            pub async fn update_fields(
                pool: &::sqlx::Pool<::sqlx::Any>,
                id: &str,
                fields: &[(&str, &str)],
            ) -> Result<bool, ::sqlx::Error> {
                if fields.is_empty() { return Ok(false); }
                let sets: Vec<String> = fields.iter().enumerate().map(|(i, (k, _))| format!("{} = ${}", k, i + 1)).collect();
                let sql = format!(
                    "UPDATE {} SET {} WHERE {} = ${}",
                    #table, sets.join(", "), stringify!(#pk), fields.len() + 1
                );
                let mut q = ::sqlx::query(&sql);
                for (_, v) in fields { q = q.bind(*v); }
                q = q.bind(id);
                let start = ::std::time::Instant::now();
                let result = q.execute(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, result.as_ref().map(|r| if r.rows_affected() > 0 { 1 } else { 0 }).unwrap_or(0), 2, result.is_err());
                result.map(|r| r.rows_affected() > 0)
            }

            /// Execute raw UPDATE with SET clause and WHERE condition.
            /// Example: `Profile::update_where(pool, "xp = xp + 10", "id = ?", &[user_id]).await?`
            pub async fn update_where(
                pool: &::sqlx::Pool<::sqlx::Any>,
                set_clause: &str,
                condition: &str,
                binds: &[&str],
            ) -> Result<u64, ::sqlx::Error> {
                let sql = format!("UPDATE {} SET {} WHERE {}", #table, set_clause, condition);
                let mut q = ::sqlx::query(&sql);
                for b in binds { q = q.bind(*b); }
                let start = ::std::time::Instant::now();
                let result = q.execute(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, result.as_ref().map(|r| r.rows_affected() as u32).unwrap_or(0), 2, result.is_err());
                result.map(|r| r.rows_affected())
            }

            /// Scalar query on this table.
            /// Example: `Profile::query_scalar_where::<i64>(pool, "CAST(COUNT(*) AS INTEGER)", "xp > ?", &["100"]).await?`
            pub async fn query_scalar_where<T: ::sqlx::Type<::sqlx::Any> + for<'r> ::sqlx::Decode<'r, ::sqlx::Any> + Send + Unpin>(
                pool: &::sqlx::Pool<::sqlx::Any>,
                select_expr: &str,
                condition: &str,
                binds: &[&str],
            ) -> Result<T, ::sqlx::Error> {
                let sql = format!("SELECT {} FROM {} WHERE {}", select_expr, #table, condition);
                let mut q = ::sqlx::query_scalar::<_, T>(&sql);
                for b in binds { q = q.bind(*b); }
                let start = ::std::time::Instant::now();
                let result = q.fetch_one(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, 1, 0, result.is_err());
                result
            }

            // ── Mixed-type bind variants (VilBind) ──

            /// INSERT with mixed-type binds.
            /// Example: `Profile::insert(pool, &["id","username","xp"], vil_args!["abc","alice",100_i64]).await?`
            pub async fn insert(
                pool: &::sqlx::Pool<::sqlx::Any>,
                columns: &[&str],
                binds: Vec<&dyn ::vil_orm::VilBind>,
            ) -> Result<u64, ::sqlx::Error> {
                let placeholders: Vec<String> = columns.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect();
                let sql = format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    #table, columns.join(", "), placeholders.join(", ")
                );
                let args = ::vil_orm::build_args(&binds);
                let start = ::std::time::Instant::now();
                let result = ::sqlx::query_with(&sql, args).execute(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, result.as_ref().map(|r| r.rows_affected() as u32).unwrap_or(0), 1, result.is_err());
                result.map(|r| r.rows_affected())
            }

            /// UPDATE with mixed-type binds.
            /// Example: `Profile::update_v(pool, "xp = xp + ?", "id = ?", vil_args![25_i64, "user-id"]).await?`
            pub async fn update_v(
                pool: &::sqlx::Pool<::sqlx::Any>,
                set_clause: &str,
                condition: &str,
                binds: Vec<&dyn ::vil_orm::VilBind>,
            ) -> Result<u64, ::sqlx::Error> {
                let sql = format!("UPDATE {} SET {} WHERE {}", #table, set_clause, condition);
                let args = ::vil_orm::build_args(&binds);
                let start = ::std::time::Instant::now();
                let result = ::sqlx::query_with(&sql, args).execute(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, result.as_ref().map(|r| r.rows_affected() as u32).unwrap_or(0), 2, result.is_err());
                result.map(|r| r.rows_affected())
            }

            /// SELECT scalar with mixed-type binds.
            /// Example: `Profile::scalar_v::<i64>(pool, "CAST(COUNT(*) AS INTEGER)", "xp > ?", vil_args![100_i64]).await?`
            pub async fn scalar_v<T: ::sqlx::Type<::sqlx::Any> + for<'r> ::sqlx::Decode<'r, ::sqlx::Any> + Send + Unpin>(
                pool: &::sqlx::Pool<::sqlx::Any>,
                select_expr: &str,
                condition: &str,
                binds: Vec<&dyn ::vil_orm::VilBind>,
            ) -> Result<T, ::sqlx::Error> {
                let sql = format!("SELECT {} FROM {} WHERE {}", select_expr, #table, condition);
                let args = ::vil_orm::build_args(&binds);
                let start = ::std::time::Instant::now();
                let result = ::sqlx::query_scalar_with::<_, T, _>(&sql, args).fetch_one(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, 1, 0, result.is_err());
                result
            }

            /// SELECT scalar optional with mixed-type binds.
            pub async fn scalar_optional_v<T: ::sqlx::Type<::sqlx::Any> + for<'r> ::sqlx::Decode<'r, ::sqlx::Any> + Send + Unpin>(
                pool: &::sqlx::Pool<::sqlx::Any>,
                select_expr: &str,
                condition: &str,
                binds: Vec<&dyn ::vil_orm::VilBind>,
            ) -> Result<Option<T>, ::sqlx::Error> {
                let sql = format!("SELECT {} FROM {} WHERE {}", select_expr, #table, condition);
                let args = ::vil_orm::build_args(&binds);
                let start = ::std::time::Instant::now();
                let result = ::sqlx::query_scalar_with::<_, T, _>(&sql, args).fetch_optional(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, if result.as_ref().ok().and_then(|r| r.as_ref()).is_some() { 1 } else { 0 }, 0, result.is_err());
                result
            }

            /// DELETE with mixed-type binds (custom condition).
            pub async fn delete_where(
                pool: &::sqlx::Pool<::sqlx::Any>,
                condition: &str,
                binds: &[&str],
            ) -> Result<u64, ::sqlx::Error> {
                let sql = format!("DELETE FROM {} WHERE {}", #table, condition);
                let mut q = ::sqlx::query(&sql);
                for b in binds { q = q.bind(*b); }
                let start = ::std::time::Instant::now();
                let result = q.execute(pool).await;
                let dur = start.elapsed().as_nanos() as u64;
                ::vil_orm::log::emit(#table, &sql, dur, result.as_ref().map(|r| r.rows_affected() as u32).unwrap_or(0), 3, result.is_err());
                result.map(|r| r.rows_affected())
            }
        }
    };

    TokenStream::from(expanded)
}

fn err(name: &syn::Ident, msg: &str) -> TokenStream {
    TokenStream::from(
        syn::Error::new_spanned(name, msg).to_compile_error(),
    )
}

// =============================================================================
// VilCrud — Auto REST endpoints from VilEntity
// =============================================================================

/// Derive macro that generates `crud_service()` returning a ServiceProcess
/// with GET /, GET /:id, POST /, PATCH /:id, DELETE /:id endpoints.
///
/// Requires `VilEntity` to be derived on the same struct.
///
/// ```ignore
/// #[derive(VilEntity, VilCrud, VilModel, sqlx::FromRow)]
/// #[vil_entity(table = "profiles")]
/// #[vil_crud(prefix = "/api/profiles")]
/// pub struct Profile {
///     #[vil_entity(pk, auto_uuid)]
///     pub id: String,
///     pub username: String,
///     pub xp: i64,
/// }
///
/// // Usage:
/// VilApp::new("app")
///     .service(Profile::crud_service(pool))
///     .run().await;
/// ```
#[proc_macro_derive(VilCrud, attributes(vil_crud, vil_entity))]
pub fn derive_vil_crud(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_lower = name.to_string().to_lowercase();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(f) => &f.named,
            _ => return err(name, "VilCrud requires named fields"),
        },
        _ => return err(name, "VilCrud only supports structs"),
    };

    // Parse #[vil_crud(prefix = "...")]
    let mut prefix = format!("/api/{}", &name_lower);
    let mut service_name = name_lower.clone();
    for attr in &input.attrs {
        if attr.path().is_ident("vil_crud") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("prefix") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    prefix = lit.value();
                }
                if meta.path.is_ident("service") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    service_name = lit.value();
                }
                Ok(())
            });
        }
    }

    // Parse #[vil_entity(table = "...")] for SQL table name
    let mut table_name = format!("{}s", &name_lower);
    let mut pk_field = format_ident!("id");
    let mut auto_uuid = false;
    let mut write_only: Vec<String> = Vec::new();

    for attr in &input.attrs {
        if attr.path().is_ident("vil_entity") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("table") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    table_name = lit.value();
                }
                Ok(())
            });
        }
    }

    for field in fields {
        let fname = field.ident.as_ref().unwrap();
        for attr in &field.attrs {
            if attr.path().is_ident("vil_entity") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("pk") { pk_field = fname.clone(); }
                    if meta.path.is_ident("auto_uuid") { auto_uuid = true; }
                    if meta.path.is_ident("write_only") { write_only.push(fname.to_string()); }
                    Ok(())
                });
            }
        }
    }

    // Collect insertable field names (exclude pk if auto_uuid, exclude auto timestamps)
    let mut insertable_fields: Vec<String> = Vec::new();
    let mut all_field_names: Vec<String> = Vec::new();
    for field in fields {
        let fname = field.ident.as_ref().unwrap().to_string();
        all_field_names.push(fname.clone());

        let is_pk = fname == pk_field.to_string();
        let mut is_auto = false;
        for attr in &field.attrs {
            if attr.path().is_ident("vil_entity") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("auto_now_add") || meta.path.is_ident("auto_now") || meta.path.is_ident("auto_uuid") {
                        is_auto = true;
                    }
                    Ok(())
                });
            }
        }
        if !is_auto && !(is_pk && auto_uuid) {
            insertable_fields.push(fname);
        }
    }

    let table = &table_name;
    let pk_str = pk_field.to_string();
    let svc_name = &service_name;

    // Build insert SQL
    let insert_cols = if auto_uuid {
        let mut cols = vec![pk_str.clone()];
        cols.extend(insertable_fields.iter().cloned());
        cols
    } else {
        insertable_fields.clone()
    };
    let insert_placeholders: Vec<String> = insert_cols.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect();
    let insert_sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table,
        insert_cols.join(", "),
        insert_placeholders.join(", ")
    );

    let list_sql = format!("SELECT * FROM {} ORDER BY {} DESC LIMIT ? OFFSET ?", table, pk_str);
    let get_sql = format!("SELECT * FROM {} WHERE {} = $1", table, pk_str);
    let delete_sql = format!("DELETE FROM {} WHERE {} = $1", table, pk_str);
    let count_sql = format!("SELECT CAST(COUNT(*) AS INTEGER) FROM {}", table);

    let expanded = quote! {
        impl #name {
            /// Returns a ServiceProcess with 5 CRUD endpoints.
            ///
            /// Endpoints: GET /, GET /:id, POST /, PATCH /:id, DELETE /:id
            pub fn crud_service(pool: ::std::sync::Arc<::vil_db_sqlx::SqlxPool>) -> ::vil_server_core::vx::service::ServiceProcess {
                use ::vil_server_core::vx::service::ServiceProcess;
                use ::vil_server_core::axum::http::Method;
                use ::vil_server_core::axum::routing::{get, post, patch, delete};

                ServiceProcess::new(#svc_name)
                    .endpoint(Method::GET, "/", get(Self::__vil_crud_list))
                    .endpoint(Method::GET, "/:id", get(Self::__vil_crud_get))
                    .endpoint(Method::POST, "/", post(Self::__vil_crud_create))
                    .endpoint(Method::DELETE, "/:id", delete(Self::__vil_crud_delete))
                    .state(pool)
            }

            /// GET / — list with pagination
            async fn __vil_crud_list(
                ctx: ::vil_server_core::vx::ctx::ServiceCtx,
                ::vil_server_core::axum::extract::Query(params): ::vil_server_core::axum::extract::Query<::std::collections::HashMap<String, String>>,
            ) -> ::vil_server_core::axum::response::Response {
                let pool = match ctx.state::<::std::sync::Arc<::vil_db_sqlx::SqlxPool>>() {
                    Ok(p) => p,
                    Err(_) => return (
                        ::vil_server_core::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "state error",
                    ).into_response(),
                };
                let page: i64 = params.get("page").and_then(|v| v.parse().ok()).unwrap_or(1);
                let per_page: i64 = params.get("per_page").and_then(|v| v.parse().ok()).unwrap_or(20).min(100);
                let offset = (page - 1) * per_page;

                let total: i64 = match ::sqlx::query_scalar(#count_sql)
                    .fetch_one(pool.inner()).await {
                    Ok(t) => t,
                    Err(_) => 0,
                };

                let data: Vec<#name> = match ::sqlx::query_as(#list_sql)
                    .bind(per_page).bind(offset)
                    .fetch_all(pool.inner()).await {
                    Ok(d) => d,
                    Err(e) => return (
                        ::vil_server_core::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("query error: {}", e),
                    ).into_response(),
                };

                let pages = if per_page > 0 { (total + per_page - 1) / per_page } else { 0 };
                let body = ::serde_json::json!({
                    "data": data,
                    "pagination": { "page": page, "per_page": per_page, "total": total, "pages": pages }
                });
                use ::vil_server_core::axum::response::IntoResponse;
                (::vil_server_core::axum::http::StatusCode::OK, ::vil_server_core::axum::Json(body)).into_response()
            }

            /// GET /:id — get by primary key
            async fn __vil_crud_get(
                ctx: ::vil_server_core::vx::ctx::ServiceCtx,
                ::vil_server_core::axum::extract::Path(id): ::vil_server_core::axum::extract::Path<String>,
            ) -> ::vil_server_core::axum::response::Response {
                let pool = match ctx.state::<::std::sync::Arc<::vil_db_sqlx::SqlxPool>>() {
                    Ok(p) => p,
                    Err(_) => return (
                        ::vil_server_core::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "state error",
                    ).into_response(),
                };
                use ::vil_server_core::axum::response::IntoResponse;
                match ::sqlx::query_as::<_, #name>(#get_sql).bind(&id).fetch_optional(pool.inner()).await {
                    Ok(Some(item)) => {
                        let body = ::serde_json::to_value(&item).unwrap_or_default();
                        (::vil_server_core::axum::http::StatusCode::OK, ::vil_server_core::axum::Json(body)).into_response()
                    }
                    Ok(None) => (::vil_server_core::axum::http::StatusCode::NOT_FOUND, "not found").into_response(),
                    Err(e) => (::vil_server_core::axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
                }
            }

            /// POST / — create
            async fn __vil_crud_create(
                ctx: ::vil_server_core::vx::ctx::ServiceCtx,
                body: ::vil_server_core::shm_extractor::ShmSlice,
            ) -> ::vil_server_core::axum::response::Response {
                let pool = match ctx.state::<::std::sync::Arc<::vil_db_sqlx::SqlxPool>>() {
                    Ok(p) => p,
                    Err(_) => return (
                        ::vil_server_core::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "state error",
                    ).into_response(),
                };
                use ::vil_server_core::axum::response::IntoResponse;

                let data: ::serde_json::Value = match body.json() {
                    Ok(d) => d,
                    Err(_) => return (::vil_server_core::axum::http::StatusCode::BAD_REQUEST, "invalid json").into_response(),
                };

                // Build insert query dynamically from JSON fields
                let id = ::uuid::Uuid::new_v4().to_string();
                let mut q = ::sqlx::query(#insert_sql);
                // Bind pk if auto_uuid
                q = q.bind(&id);
                // Bind remaining insertable fields from JSON body
                #(
                    {
                        let field_name = #insertable_fields;
                        let val = data.get(field_name).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        q = q.bind(val);
                    }
                )*

                match q.execute(pool.inner()).await {
                    Ok(_) => {
                        match ::sqlx::query_as::<_, #name>(#get_sql).bind(&id).fetch_one(pool.inner()).await {
                            Ok(created) => {
                                let body = ::serde_json::to_value(&created).unwrap_or_default();
                                (::vil_server_core::axum::http::StatusCode::CREATED, ::vil_server_core::axum::Json(body)).into_response()
                            }
                            Err(e) => (::vil_server_core::axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
                        }
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("UNIQUE") {
                            (::vil_server_core::axum::http::StatusCode::CONFLICT, "duplicate entry").into_response()
                        } else {
                            (::vil_server_core::axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
                        }
                    }
                }
            }

            /// DELETE /:id
            async fn __vil_crud_delete(
                ctx: ::vil_server_core::vx::ctx::ServiceCtx,
                ::vil_server_core::axum::extract::Path(id): ::vil_server_core::axum::extract::Path<String>,
            ) -> ::vil_server_core::axum::response::Response {
                let pool = match ctx.state::<::std::sync::Arc<::vil_db_sqlx::SqlxPool>>() {
                    Ok(p) => p,
                    Err(_) => return (
                        ::vil_server_core::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "state error",
                    ).into_response(),
                };
                use ::vil_server_core::axum::response::IntoResponse;
                match ::sqlx::query(#delete_sql).bind(&id).execute(pool.inner()).await {
                    Ok(r) if r.rows_affected() > 0 => ::vil_server_core::axum::http::StatusCode::NO_CONTENT.into_response(),
                    Ok(_) => (::vil_server_core::axum::http::StatusCode::NOT_FOUND, "not found").into_response(),
                    Err(e) => (::vil_server_core::axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)).into_response(),
                }
            }
        }
    };

    TokenStream::from(expanded)
}
