//! Service Generator — Generates 5 CRUD handler functions per table.
//!
//! Each table gets: list, get_by_id, create, update, delete.
//! Uses VilEntity methods + VilQuery builder for all DB operations.

use super::model_gen::{to_pascal_case, is_reserved_word};
use super::schema_parser::TableMeta;

/// Get the Rust field name for a column (appends _ for reserved words).
fn rust_field(name: &str) -> String {
    if is_reserved_word(name) {
        format!("{}_", name)
    } else {
        name.to_string()
    }
}

/// Generate the complete service file for a table.
pub fn generate_service_file(table: &TableMeta) -> String {
    let struct_name = to_pascal_case(&table.name);
    let snake = &table.name;
    let mut out = String::with_capacity(4096);

    // Imports (glob import covers {Table}Pk for composite PK tables)
    out.push_str(&format!(
        "use crate::error::AppError;\n\
         use crate::models::{}::*;\n\
         use crate::AppState;\n\
         use vil_server::prelude::*;\n\n",
        snake
    ));

    // list handler
    out.push_str(&gen_list(table, &struct_name, snake));
    out.push_str("\n");

    // get_by_id handler
    out.push_str(&gen_get_by_id(table, &struct_name, snake));
    out.push_str("\n");

    // create handler
    out.push_str(&gen_create(table, &struct_name, snake));
    out.push_str("\n");

    // update handler
    out.push_str(&gen_update(table, &struct_name, snake));
    out.push_str("\n");

    // delete handler
    out.push_str(&gen_delete(table, &struct_name, snake));

    out
}

/// GET /{table} — list with VilQuery select projection
fn gen_list(table: &TableMeta, struct_name: &str, _snake: &str) -> String {
    let list_cols = table.list_columns();
    let has_list_item = list_cols.len() < table.columns.len();

    // Build column list for select projection
    let col_names: Vec<String> = list_cols
        .iter()
        .filter(|c| !c.is_sensitive())
        .map(|c| format!("\"{}\"", c.name))
        .collect();

    let return_type = if has_list_item {
        format!("{}ListItem", struct_name)
    } else {
        struct_name.to_string()
    };

    // Detect if table has created_at for ordering
    let has_created_at = table.columns.iter().any(|c| c.name == "created_at");
    let order_col = if has_created_at {
        "created_at"
    } else {
        table.first_pk()
    };

    format!(
        "#[vil_handler]\n\
         pub async fn list(ctx: ServiceCtx) -> Result<VilResponse<Vec<{return_type}>>, AppError> {{\n\
         {indent}let state = ctx.state::<AppState>().map_err(|_| AppError::Internal(\"state\".into()))?;\n\
         {indent}let items = {struct_name}::q()\n\
         {indent2}.select(&[{cols}])\n\
         {indent2}.order_by_desc(\"{order}\")\n\
         {indent2}.limit(100)\n\
         {indent2}.fetch_all::<{return_type}>(state.pool.inner())\n\
         {indent2}.await?;\n\
         {indent}Ok(VilResponse::ok(items))\n\
         }}\n",
        return_type = return_type,
        struct_name = struct_name,
        cols = col_names.join(", "),
        order = order_col,
        indent = "    ",
        indent2 = "        ",
    )
}

/// GET /{table}/:id — find by primary key
fn gen_get_by_id(table: &TableMeta, struct_name: &str, _snake: &str) -> String {
    if table.is_composite_pk() {
        // Composite PK: Path extracts a tuple, then query with where_eq + and_eq
        let pk_struct = format!("{}Pk", struct_name);
        let pks = &table.primary_keys;

        let mut where_chain = format!(
            ".where_eq(\"{}\", &pk.{})",
            pks[0],
            rust_field(&pks[0])
        );
        for pk_col in &pks[1..] {
            where_chain.push_str(&format!(
                "\n        .and_eq(\"{}\", &pk.{})",
                pk_col,
                rust_field(pk_col)
            ));
        }

        // Build path tuple type: (String, String, ...)
        let path_types: Vec<&str> = pks.iter().map(|_| "String").collect();
        let path_type = format!("({})", path_types.join(", "));
        let path_fields: Vec<String> = pks.iter().map(|p| rust_field(p)).collect();
        let path_destructure = format!("({})", path_fields.join(", "));

        // Build pk struct construction
        let mut pk_fields = String::new();
        for pk_col in pks {
            pk_fields.push_str(&format!(
                "        {}: {},\n",
                rust_field(pk_col),
                rust_field(pk_col)
            ));
        }

        format!(
            "#[vil_handler]\n\
             pub async fn get_by_pk(\n\
             {indent}ctx: ServiceCtx,\n\
             {indent}Path({path_destructure}): Path<{path_type}>,\n\
             ) -> Result<VilResponse<{struct_name}>, AppError> {{\n\
             {indent}let state = ctx.state::<AppState>().map_err(|_| AppError::Internal(\"state\".into()))?;\n\
             {indent}let pk = {pk_struct} {{\n\
             {pk_fields}\
             {indent}}};\n\
             {indent}let item = {struct_name}::q()\n\
             {indent2}.select(&[\"*\"])\n\
             {indent2}{where_chain}\n\
             {indent2}.fetch_optional::<{struct_name}>(state.pool.inner())\n\
             {indent2}.await?\n\
             {indent2}.ok_or_else(|| AppError::NotFound(\"{struct_name} not found\".into()))?;\n\
             {indent}Ok(VilResponse::ok(item))\n\
             }}\n",
            struct_name = struct_name,
            pk_struct = pk_struct,
            pk_fields = pk_fields,
            path_destructure = path_destructure,
            path_type = path_type,
            where_chain = where_chain,
            indent = "    ",
            indent2 = "        ",
        )
    } else {
        format!(
            "#[vil_handler]\n\
             pub async fn get_by_id(\n\
             {indent}ctx: ServiceCtx,\n\
             {indent}Path(id): Path<String>,\n\
             ) -> Result<VilResponse<{struct_name}>, AppError> {{\n\
             {indent}let state = ctx.state::<AppState>().map_err(|_| AppError::Internal(\"state\".into()))?;\n\
             {indent}let item = {struct_name}::find_by_id(state.pool.inner(), &id)\n\
             {indent2}.await?\n\
             {indent2}.ok_or_else(|| AppError::NotFound(\"{struct_name} not found\".into()))?;\n\
             {indent}Ok(VilResponse::ok(item))\n\
             }}\n",
            struct_name = struct_name,
            indent = "    ",
            indent2 = "        ",
        )
    }
}

/// POST /{table} — create via VilQuery insert
fn gen_create(table: &TableMeta, struct_name: &str, _snake: &str) -> String {
    let create_type = format!("Create{}Request", struct_name);
    let is_composite = table.is_composite_pk();

    // Build insert columns + value calls
    let mut insert_cols = Vec::new();
    let mut value_calls = Vec::new();

    if is_composite {
        // Composite PK: user provides all PK columns in the request body
        for pk_col in &table.primary_keys {
            insert_cols.push(format!("\"{}\"", pk_col));
            value_calls.push(format!(
                "        .value(req.{}.clone())",
                rust_field(pk_col)
            ));
        }
    } else {
        // Single PK: auto UUID
        let pk = table.first_pk();
        insert_cols.push(format!("\"{}\"", pk));
        value_calls.push("        .value(id.clone())".to_string());
    }

    for col in &table.columns {
        if col.is_primary_key || col.is_auto_timestamp() {
            continue;
        }
        // Skip columns with DEFAULT — let DB fill them
        if col.default_value.is_some() {
            continue;
        }

        insert_cols.push(format!("\"{}\"", col.name));

        // Determine how to bind the value
        if col.nullable {
            // Optional field in CreateRequest
            match col.rust_type() {
                "String" => value_calls.push(format!(
                    "        .value_opt_str(req.{}.clone())",
                    rust_field(&col.name)
                )),
                "i64" => value_calls.push(format!(
                    "        .value_opt_i64(req.{})",
                    rust_field(&col.name)
                )),
                "f64" => value_calls.push(format!(
                    "        .value_opt_f64(req.{})",
                    rust_field(&col.name)
                )),
                _ => value_calls.push(format!(
                    "        .value_opt_str(req.{}.clone())",
                    rust_field(&col.name)
                )),
            }
        } else {
            // Required field
            match col.rust_type() {
                "String" => value_calls.push(format!(
                    "        .value(req.{}.clone())",
                    rust_field(&col.name)
                )),
                "i64" => value_calls.push(format!(
                    "        .value(req.{})",
                    rust_field(&col.name)
                )),
                "f64" => value_calls.push(format!(
                    "        .value(req.{})",
                    rust_field(&col.name)
                )),
                _ => value_calls.push(format!(
                    "        .value(req.{}.clone())",
                    rust_field(&col.name)
                )),
            }
        }
    }

    if is_composite {
        // Composite PK: no auto UUID, re-fetch via query
        let pks = &table.primary_keys;
        let mut where_chain = format!(
            ".where_eq(\"{}\", &req.{})",
            pks[0],
            rust_field(&pks[0])
        );
        for pk_col in &pks[1..] {
            where_chain.push_str(&format!(
                "\n        .and_eq(\"{}\", &req.{})",
                pk_col,
                rust_field(pk_col)
            ));
        }

        format!(
            "#[vil_handler]\n\
             pub async fn create(\n\
             {indent}ctx: ServiceCtx,\n\
             {indent}body: ShmSlice,\n\
             ) -> Result<VilResponse<{struct_name}>, AppError> {{\n\
             {indent}let state = ctx.state::<AppState>().map_err(|_| AppError::Internal(\"state\".into()))?;\n\
             {indent}let req: {create_type} = body.json().map_err(|_| AppError::Validation(\"Invalid JSON\".into()))?;\n\
             \n\
             {indent}{struct_name}::q()\n\
             {indent2}.insert_columns(&[{cols}])\n\
             {values}\n\
             {indent2}.execute(state.pool.inner())\n\
             {indent2}.await?;\n\
             \n\
             {indent}let created = {struct_name}::q()\n\
             {indent2}.select(&[\"*\"])\n\
             {indent2}{where_chain}\n\
             {indent2}.fetch_optional::<{struct_name}>(state.pool.inner())\n\
             {indent2}.await?\n\
             {indent2}.ok_or_else(|| AppError::Internal(\"Created but not found\".into()))?;\n\
             {indent}Ok(VilResponse::created(created))\n\
             }}\n",
            struct_name = struct_name,
            create_type = create_type,
            cols = insert_cols.join(", "),
            values = value_calls.join("\n"),
            where_chain = where_chain,
            indent = "    ",
            indent2 = "        ",
        )
    } else {
        format!(
            "#[vil_handler]\n\
             pub async fn create(\n\
             {indent}ctx: ServiceCtx,\n\
             {indent}body: ShmSlice,\n\
             ) -> Result<VilResponse<{struct_name}>, AppError> {{\n\
             {indent}let state = ctx.state::<AppState>().map_err(|_| AppError::Internal(\"state\".into()))?;\n\
             {indent}let req: {create_type} = body.json().map_err(|_| AppError::Validation(\"Invalid JSON\".into()))?;\n\
             {indent}let id = uuid::Uuid::new_v4().to_string();\n\
             \n\
             {indent}{struct_name}::q()\n\
             {indent2}.insert_columns(&[{cols}])\n\
             {values}\n\
             {indent2}.execute(state.pool.inner())\n\
             {indent2}.await?;\n\
             \n\
             {indent}let created = {struct_name}::find_by_id(state.pool.inner(), &id)\n\
             {indent2}.await?\n\
             {indent2}.ok_or_else(|| AppError::Internal(\"Created but not found\".into()))?;\n\
             {indent}Ok(VilResponse::created(created))\n\
             }}\n",
            struct_name = struct_name,
            create_type = create_type,
            cols = insert_cols.join(", "),
            values = value_calls.join("\n"),
            indent = "    ",
            indent2 = "        ",
        )
    }
}

/// PUT /{table}/:id — update via VilQuery set_optional
fn gen_update(table: &TableMeta, struct_name: &str, _snake: &str) -> String {
    let update_type = format!("Update{}Request", struct_name);

    // Build set_optional calls for each updatable column
    let mut set_calls = Vec::new();
    for col in &table.columns {
        if col.is_primary_key || col.is_auto_timestamp() {
            continue;
        }

        match col.rust_type() {
            "String" => set_calls.push(format!(
                "        .set_optional(\"{}\", req.{}.as_deref())",
                col.name, rust_field(&col.name)
            )),
            "i64" => set_calls.push(format!(
                "        .set_optional_i64(\"{}\", req.{})",
                col.name, rust_field(&col.name)
            )),
            "f64" => set_calls.push(format!(
                "        .set_optional_f64(\"{}\", req.{})",
                col.name, rust_field(&col.name)
            )),
            _ => set_calls.push(format!(
                "        .set_optional(\"{}\", req.{}.as_deref())",
                col.name, rust_field(&col.name)
            )),
        }
    }

    // Add updated_at if table has it
    let has_updated_at = table.columns.iter().any(|c| c.is_updated_at());
    if has_updated_at {
        set_calls.push("        .set_raw(\"updated_at\", \"datetime('now')\")".to_string());
    }

    if table.is_composite_pk() {
        let pk_struct = format!("{}Pk", struct_name);
        let pks = &table.primary_keys;

        let mut where_chain = format!(
            ".where_eq(\"{}\", &pk.{})",
            pks[0],
            rust_field(&pks[0])
        );
        for pk_col in &pks[1..] {
            where_chain.push_str(&format!(
                "\n        .and_eq(\"{}\", &pk.{})",
                pk_col,
                rust_field(pk_col)
            ));
        }

        // Build path tuple
        let path_types: Vec<&str> = pks.iter().map(|_| "String").collect();
        let path_type = format!("({})", path_types.join(", "));
        let path_fields: Vec<String> = pks.iter().map(|p| rust_field(p)).collect();
        let path_destructure = format!("({})", path_fields.join(", "));

        let mut pk_fields = String::new();
        for pk_col in pks {
            pk_fields.push_str(&format!(
                "        {}: {},\n",
                rust_field(pk_col),
                rust_field(pk_col)
            ));
        }

        // Re-fetch via query
        let mut refetch_where = format!(
            ".where_eq(\"{}\", &pk.{})",
            pks[0],
            rust_field(&pks[0])
        );
        for pk_col in &pks[1..] {
            refetch_where.push_str(&format!(
                "\n        .and_eq(\"{}\", &pk.{})",
                pk_col,
                rust_field(pk_col)
            ));
        }

        format!(
            "#[vil_handler]\n\
             pub async fn update(\n\
             {indent}ctx: ServiceCtx,\n\
             {indent}Path({path_destructure}): Path<{path_type}>,\n\
             {indent}body: ShmSlice,\n\
             ) -> Result<VilResponse<{struct_name}>, AppError> {{\n\
             {indent}let state = ctx.state::<AppState>().map_err(|_| AppError::Internal(\"state\".into()))?;\n\
             {indent}let req: {update_type} = body.json().map_err(|_| AppError::Validation(\"Invalid JSON\".into()))?;\n\
             {indent}let pk = {pk_struct} {{\n\
             {pk_fields}\
             {indent}}};\n\
             \n\
             {indent}{struct_name}::q()\n\
             {indent2}.update()\n\
             {sets}\n\
             {indent2}{where_chain}\n\
             {indent2}.execute(state.pool.inner())\n\
             {indent2}.await?;\n\
             \n\
             {indent}let updated = {struct_name}::q()\n\
             {indent2}.select(&[\"*\"])\n\
             {indent2}{refetch_where}\n\
             {indent2}.fetch_optional::<{struct_name}>(state.pool.inner())\n\
             {indent2}.await?\n\
             {indent2}.ok_or_else(|| AppError::NotFound(\"{struct_name} not found\".into()))?;\n\
             {indent}Ok(VilResponse::ok(updated))\n\
             }}\n",
            struct_name = struct_name,
            update_type = update_type,
            pk_struct = pk_struct,
            pk_fields = pk_fields,
            path_destructure = path_destructure,
            path_type = path_type,
            where_chain = where_chain,
            refetch_where = refetch_where,
            sets = set_calls.join("\n"),
            indent = "    ",
            indent2 = "        ",
        )
    } else {
        format!(
            "#[vil_handler]\n\
             pub async fn update(\n\
             {indent}ctx: ServiceCtx,\n\
             {indent}Path(id): Path<String>,\n\
             {indent}body: ShmSlice,\n\
             ) -> Result<VilResponse<{struct_name}>, AppError> {{\n\
             {indent}let state = ctx.state::<AppState>().map_err(|_| AppError::Internal(\"state\".into()))?;\n\
             {indent}let req: {update_type} = body.json().map_err(|_| AppError::Validation(\"Invalid JSON\".into()))?;\n\
             \n\
             {indent}{struct_name}::q()\n\
             {indent2}.update()\n\
             {sets}\n\
             {indent2}.where_eq(\"{pk}\", &id)\n\
             {indent2}.execute(state.pool.inner())\n\
             {indent2}.await?;\n\
             \n\
             {indent}let updated = {struct_name}::find_by_id(state.pool.inner(), &id)\n\
             {indent2}.await?\n\
             {indent2}.ok_or_else(|| AppError::NotFound(\"{struct_name} not found\".into()))?;\n\
             {indent}Ok(VilResponse::ok(updated))\n\
             }}\n",
            struct_name = struct_name,
            update_type = update_type,
            pk = table.first_pk(),
            sets = set_calls.join("\n"),
            indent = "    ",
            indent2 = "        ",
        )
    }
}

/// DELETE /{table}/:id — delete by primary key
fn gen_delete(table: &TableMeta, struct_name: &str, _snake: &str) -> String {
    if table.is_composite_pk() {
        let pks = &table.primary_keys;

        // Build path tuple
        let path_types: Vec<&str> = pks.iter().map(|_| "String").collect();
        let path_type = format!("({})", path_types.join(", "));
        let path_fields: Vec<String> = pks.iter().map(|p| rust_field(p)).collect();
        let path_destructure = format!("({})", path_fields.join(", "));

        let mut where_chain = format!(
            ".where_eq(\"{}\", &{})",
            pks[0],
            rust_field(&pks[0])
        );
        for pk_col in &pks[1..] {
            where_chain.push_str(&format!(
                "\n        .and_eq(\"{}\", &{})",
                pk_col,
                rust_field(pk_col)
            ));
        }

        format!(
            "#[vil_handler]\n\
             pub async fn delete(\n\
             {indent}ctx: ServiceCtx,\n\
             {indent}Path({path_destructure}): Path<{path_type}>,\n\
             ) -> Result<VilResponse<serde_json::Value>, AppError> {{\n\
             {indent}let state = ctx.state::<AppState>().map_err(|_| AppError::Internal(\"state\".into()))?;\n\
             {indent}let result = {struct_name}::q()\n\
             {indent2}.delete()\n\
             {indent2}{where_chain}\n\
             {indent2}.execute(state.pool.inner())\n\
             {indent2}.await?;\n\
             {indent}let deleted = result > 0;\n\
             {indent}if deleted {{\n\
             {indent2}Ok(VilResponse::ok(serde_json::json!({{\"deleted\": true}})))\n\
             {indent}}} else {{\n\
             {indent2}Err(AppError::NotFound(\"{struct_name} not found\".into()))\n\
             {indent}}}\n\
             }}\n",
            struct_name = struct_name,
            path_destructure = path_destructure,
            path_type = path_type,
            where_chain = where_chain,
            indent = "    ",
            indent2 = "        ",
        )
    } else {
        format!(
            "#[vil_handler]\n\
             pub async fn delete(\n\
             {indent}ctx: ServiceCtx,\n\
             {indent}Path(id): Path<String>,\n\
             ) -> Result<VilResponse<serde_json::Value>, AppError> {{\n\
             {indent}let state = ctx.state::<AppState>().map_err(|_| AppError::Internal(\"state\".into()))?;\n\
             {indent}let existed = {struct_name}::delete(state.pool.inner(), &id).await?;\n\
             {indent}if existed {{\n\
             {indent2}Ok(VilResponse::ok(serde_json::json!({{\"deleted\": true, \"id\": id}})))\n\
             {indent}}} else {{\n\
             {indent2}Err(AppError::NotFound(\"{struct_name} not found\".into()))\n\
             {indent}}}\n\
             }}\n",
            struct_name = struct_name,
            indent = "    ",
            indent2 = "        ",
        )
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orm::schema_parser;

    #[test]
    fn test_generate_service_profile() {
        let sql = r#"
CREATE TABLE profiles (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE,
    full_name TEXT,
    xp INTEGER DEFAULT 0,
    password_hash TEXT NOT NULL,
    created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
        "#;
        let tables = schema_parser::parse_schema(sql);
        let output = generate_service_file(&tables[0]);

        // list handler
        assert!(output.contains("#[vil_handler]"));
        assert!(output.contains("pub async fn list("));
        assert!(output.contains("Profile::q()"));
        assert!(output.contains(".select(&["));
        assert!(output.contains(".order_by_desc(\"created_at\")"));
        assert!(output.contains(".limit(100)"));
        assert!(output.contains("fetch_all::<"));

        // get handler
        assert!(output.contains("pub async fn get_by_id("));
        assert!(output.contains("Profile::find_by_id("));
        assert!(output.contains("Path(id): Path<String>"));

        // create handler
        assert!(output.contains("pub async fn create("));
        assert!(output.contains("CreateProfileRequest"));
        assert!(output.contains("insert_columns"));
        assert!(output.contains(".value(id.clone())"));
        assert!(output.contains("uuid::Uuid::new_v4()"));

        // update handler
        assert!(output.contains("pub async fn update("));
        assert!(output.contains("UpdateProfileRequest"));
        assert!(output.contains(".set_optional("));
        assert!(output.contains("set_raw(\"updated_at\", \"datetime('now')\")"));
        assert!(output.contains(".where_eq(\"id\", &id)"));

        // delete handler
        assert!(output.contains("pub async fn delete("));
        assert!(output.contains("Profile::delete("));
        assert!(output.contains("\"deleted\": true"));

        // password_hash excluded from list SELECT (sensitive), but present in create INSERT
        let list_section = output.split("pub async fn get_by_id").next().unwrap();
        assert!(!list_section.contains("\"password_hash\""), "password_hash should not be in list projection");
    }

    #[test]
    fn test_generate_service_nullable_fields() {
        let sql = r#"
CREATE TABLE predictions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    predicted_value REAL,
    confidence REAL,
    breakdown TEXT,
    created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
        "#;
        let tables = schema_parser::parse_schema(sql);
        let output = generate_service_file(&tables[0]);

        // Nullable REAL → value_opt_f64
        assert!(output.contains("value_opt_f64(req.predicted_value)"));
        assert!(output.contains("value_opt_f64(req.confidence)"));
        // Nullable TEXT → value_opt_str
        assert!(output.contains("value_opt_str(req.breakdown.clone())"));
        // NOT NULL TEXT → value(req.user_id.clone())
        assert!(output.contains("value(req.user_id.clone())"));

        // Update: f64 → set_optional_f64
        assert!(output.contains("set_optional_f64(\"predicted_value\""));
        assert!(output.contains("set_optional(\"breakdown\""));
    }

    #[test]
    fn test_toefl_all_services() {
        let sql = std::fs::read_to_string(
            "/home/abraham/Aplikasi-Ibrohim/new-toefl-quiz/src/db/migrations/001_initial_schema.sql"
        ).expect("read schema");
        let tables = schema_parser::parse_schema(&sql);

        println!("\n=== Generated services for {} tables ===\n", tables.len());
        for table in &tables {
            let output = generate_service_file(table);
            let struct_name = to_pascal_case(&table.name);

            // Every service must have 5 handlers
            assert!(output.contains("pub async fn list("), "Missing list for {}", table.name);
            if table.is_composite_pk() {
                assert!(output.contains("pub async fn get_by_pk("), "Missing get_by_pk for {}", table.name);
            } else {
                assert!(output.contains("pub async fn get_by_id("), "Missing get_by_id for {}", table.name);
            }
            assert!(output.contains("pub async fn create("), "Missing create for {}", table.name);
            assert!(output.contains("pub async fn update("), "Missing update for {}", table.name);
            assert!(output.contains("pub async fn delete("), "Missing delete for {}", table.name);

            // Must use VilORM
            assert!(output.contains(&format!("{}::q()", struct_name)),
                "Missing VilQuery for {}", table.name);
            if !table.is_composite_pk() {
                assert!(output.contains(&format!("{}::find_by_id(", struct_name)),
                    "Missing find_by_id for {}", table.name);
                assert!(output.contains(&format!("{}::delete(", struct_name)),
                    "Missing delete call for {}", table.name);
            }

            let lines = output.lines().count();
            println!("  {} → {}_svc.rs ({} lines, 5 handlers)", table.name, table.name, lines);
        }
    }
}
