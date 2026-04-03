//! Model Generator — Generates VilEntity Rust structs from TableMeta.
//!
//! For each table produces:
//! 1. `{Table}` — main entity struct with #[derive(VilEntity)]
//! 2. `{Table}ListItem` — slim projection (excludes large TEXT columns)
//! 3. `Create{Table}Request` — input struct for POST (excludes PK + auto fields)
//! 4. `Update{Table}Request` — input struct for PUT (all fields Option<T>)

use super::schema_parser::{ColumnMeta, TableMeta};

/// Generate the complete model file for a table.
pub fn generate_model_file(table: &TableMeta) -> String {
    let struct_name = to_pascal_case(&table.name);
    let mut out = String::with_capacity(2048);

    // Header
    out.push_str("use serde::{Deserialize, Serialize};\n");
    out.push_str("use vil_orm_derive::VilEntity;\n");
    out.push_str("use vil_server::prelude::VilModel;\n");
    out.push_str("\n");

    // Main entity struct
    out.push_str(&generate_entity_struct(table, &struct_name));
    out.push_str("\n");

    // Composite PK struct (e.g., UserSavedEssayPk)
    if table.is_composite_pk() {
        out.push_str(&generate_pk_struct(table, &struct_name));
        out.push_str("\n");
    }

    // ListItem projection struct
    let list_cols = table.list_columns();
    if list_cols.len() < table.columns.len() {
        out.push_str(&generate_list_item_struct(table, &struct_name, &list_cols));
        out.push_str("\n");
    }

    // CreateRequest struct
    out.push_str(&generate_create_request(table, &struct_name));
    out.push_str("\n");

    // UpdateRequest struct
    out.push_str(&generate_update_request(table, &struct_name));

    out
}

/// Generate the main #[derive(VilEntity)] struct.
fn generate_entity_struct(table: &TableMeta, struct_name: &str) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, VilModel, VilEntity)]\n"
    ));
    out.push_str(&format!("#[vil_entity(table = \"{}\")]\n", table.name));
    out.push_str(&format!("pub struct {} {{\n", struct_name));

    for col in &table.columns {
        // vil_entity attributes
        let mut attrs = Vec::new();
        if col.is_primary_key {
            attrs.push("pk".to_string());
        }
        if col.is_unique && !col.is_primary_key {
            attrs.push("unique".to_string());
        }
        if col.is_created_at() {
            attrs.push("auto_now_add".to_string());
        }
        if col.is_updated_at() {
            attrs.push("auto_now".to_string());
        }

        if !attrs.is_empty() {
            out.push_str(&format!("    #[vil_entity({})]\n", attrs.join(", ")));
        }

        // serde skip for sensitive fields
        if col.is_sensitive() {
            out.push_str("    #[serde(skip_serializing)]\n");
        }

        // sqlx rename for reserved words
        if is_reserved_word(&col.name) {
            out.push_str(&format!(
                "    #[sqlx(rename = \"{}\")]\n    #[serde(rename = \"{}\")]\n",
                col.name, col.name
            ));
            out.push_str(&format!(
                "    pub {}_: {},\n",
                col.name,
                col.rust_type_full()
            ));
        } else {
            // FK comment
            if let Some(ref fk) = col.references {
                out.push_str(&format!("    // FK → {}.{}\n", fk.table, fk.column));
            }
            out.push_str(&format!("    pub {}: {},\n", col.name, col.rust_type_full()));
        }
    }

    out.push_str("}\n");
    out
}

/// Generate a slim ListItem struct for list endpoints.
fn generate_list_item_struct(
    _table: &TableMeta,
    struct_name: &str,
    list_cols: &[&ColumnMeta],
) -> String {
    let mut out = String::new();
    let list_name = format!("{}ListItem", struct_name);

    out.push_str(&format!(
        "/// Slim projection for list endpoints (excludes large text fields).\n"
    ));
    out.push_str(&format!(
        "#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, VilModel)]\n"
    ));
    out.push_str(&format!("pub struct {} {{\n", list_name));

    for col in list_cols {
        if col.is_sensitive() {
            continue; // Skip password_hash etc in list views
        }
        if is_reserved_word(&col.name) {
            out.push_str(&format!(
                "    #[sqlx(rename = \"{}\")]\n    #[serde(rename = \"{}\")]\n",
                col.name, col.name
            ));
            out.push_str(&format!("    pub {}_: {},\n", col.name, col.rust_type_full()));
        } else {
            out.push_str(&format!("    pub {}: {},\n", col.name, col.rust_type_full()));
        }
    }

    out.push_str("}\n");
    out
}

/// Generate CreateRequest struct for POST endpoints.
fn generate_create_request(table: &TableMeta, struct_name: &str) -> String {
    let mut out = String::new();
    let req_name = format!("Create{}Request", struct_name);

    out.push_str(&format!("#[derive(Debug, Deserialize)]\n"));
    out.push_str(&format!("pub struct {} {{\n", req_name));

    for col in &table.columns {
        // For single PK: skip PK (auto UUID). For composite PK: include PK columns.
        if col.is_primary_key && !table.is_composite_pk() {
            continue;
        }
        if col.is_auto_timestamp() {
            continue;
        }

        // Make fields that have DEFAULT values optional in create request
        let rust_type = if col.default_value.is_some() || col.nullable {
            format!("Option<{}>", col.rust_type())
        } else {
            col.rust_type().to_string()
        };

        let field_name = if is_reserved_word(&col.name) {
            format!("{}_", col.name)
        } else {
            col.name.clone()
        };
        out.push_str(&format!("    pub {}: {},\n", field_name, rust_type));
    }

    out.push_str("}\n");
    out
}

/// Generate UpdateRequest struct for PUT endpoints (all fields Optional).
fn generate_update_request(table: &TableMeta, struct_name: &str) -> String {
    let mut out = String::new();
    let req_name = format!("Update{}Request", struct_name);

    out.push_str(&format!("#[derive(Debug, Deserialize)]\n"));
    out.push_str(&format!("pub struct {} {{\n", req_name));

    for col in &table.columns {
        // Skip PK and auto-timestamps — can't be updated
        if col.is_primary_key {
            continue;
        }
        if col.is_auto_timestamp() {
            continue;
        }

        let base_type = col.rust_type();
        let field_name = if is_reserved_word(&col.name) {
            format!("{}_", col.name)
        } else {
            col.name.clone()
        };
        out.push_str(&format!("    pub {}: Option<{}>,\n", field_name, base_type));
    }

    out.push_str("}\n");
    out
}

/// Generate a composite PK struct for tables with multi-column primary keys.
/// E.g., `UserSavedEssayPk { user_id: String, essay_id: String }`
fn generate_pk_struct(table: &TableMeta, struct_name: &str) -> String {
    let mut out = String::new();
    let pk_name = format!("{}Pk", struct_name);

    out.push_str("#[derive(Debug, Deserialize)]\n");
    out.push_str(&format!("pub struct {} {{\n", pk_name));

    for pk_col in &table.primary_keys {
        let field = if is_reserved_word(pk_col) {
            format!("{}_", pk_col)
        } else {
            pk_col.clone()
        };
        out.push_str(&format!("    pub {}: String,\n", field));
    }

    out.push_str("}\n");
    out
}

// =============================================================================
// Helpers
// =============================================================================

/// Convert snake_case to PascalCase.
/// "quiz_results" → "QuizResult" (singularize by removing trailing 's')
pub fn to_pascal_case(name: &str) -> String {
    let pascal: String = name
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect();

    // Simple singularization: remove trailing 's' for table names
    // But not for words ending in 'ss' (e.g., "address" stays)
    if pascal.ends_with('s') && !pascal.ends_with("ss") && pascal.len() > 3 {
        // Handle "ies" → "y" (e.g., "entries" → "entry")
        if pascal.ends_with("ies") {
            return format!("{}y", &pascal[..pascal.len() - 3]);
        }
        // Handle "ses" → "s" (not common in DB tables)
        if pascal.ends_with("ses") {
            return pascal[..pascal.len() - 2].to_string();
        }
        pascal[..pascal.len() - 1].to_string()
    } else {
        pascal
    }
}

/// Check if column name is a Rust reserved word.
pub fn is_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "type" | "match" | "ref" | "self" | "super" | "crate" | "mod" | "fn" | "pub"
            | "use" | "let" | "mut" | "const" | "static" | "extern" | "as" | "break"
            | "continue" | "else" | "for" | "if" | "impl" | "in" | "loop" | "move"
            | "return" | "struct" | "trait" | "where" | "while" | "async" | "await"
            | "dyn" | "abstract" | "become" | "box" | "do" | "final" | "macro"
            | "override" | "priv" | "typeof" | "unsized" | "virtual" | "yield"
    )
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orm::schema_parser;

    #[test]
    fn test_pascal_case() {
        assert_eq!(to_pascal_case("profiles"), "Profile");
        assert_eq!(to_pascal_case("quiz_results"), "QuizResult");
        assert_eq!(to_pascal_case("ai_token_usage"), "AiTokenUsage");
        assert_eq!(to_pascal_case("admin_users"), "AdminUser");
        assert_eq!(to_pascal_case("circles"), "Circle");
        assert_eq!(to_pascal_case("daily_bites"), "DailyBite");
        assert_eq!(to_pascal_case("friends"), "Friend");
        assert_eq!(to_pascal_case("app_logs"), "AppLog");
        assert_eq!(to_pascal_case("blog_posts"), "BlogPost");
        assert_eq!(to_pascal_case("peer_review_submissions"), "PeerReviewSubmission");
    }

    #[test]
    fn test_generate_profile_model() {
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
        let output = generate_model_file(&tables[0]);

        // Entity struct
        assert!(output.contains("#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, VilModel, VilEntity)]"));
        assert!(output.contains("#[vil_entity(table = \"profiles\")]"));
        assert!(output.contains("pub struct Profile {"));
        assert!(output.contains("#[vil_entity(pk)]"));
        assert!(output.contains("pub id: String,"));
        assert!(output.contains("#[vil_entity(unique)]"));
        assert!(output.contains("pub username: Option<String>,"));
        assert!(output.contains("#[serde(skip_serializing)]"));
        assert!(output.contains("pub password_hash: String,"));
        assert!(output.contains("#[vil_entity(auto_now_add)]"));
        assert!(output.contains("#[vil_entity(auto_now)]"));
        assert!(output.contains("pub xp: i64,"));

        // CreateRequest
        assert!(output.contains("pub struct CreateProfileRequest {"));
        assert!(!output.contains("CreateProfileRequest {\n    pub id:")); // no PK in create
        assert!(output.contains("pub username: Option<String>,")); // nullable → Option
        assert!(output.contains("pub password_hash: String,")); // NOT NULL → required

        // UpdateRequest
        assert!(output.contains("pub struct UpdateProfileRequest {"));
        assert!(output.contains("pub username: Option<String>,")); // all Optional
        assert!(output.contains("pub xp: Option<i64>,"));
    }

    #[test]
    fn test_reserved_word_column() {
        let sql = r#"
CREATE TABLE notifications (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    message TEXT NOT NULL
);
        "#;
        let tables = schema_parser::parse_schema(sql);
        let output = generate_model_file(&tables[0]);

        assert!(output.contains("#[sqlx(rename = \"type\")]"));
        assert!(output.contains("#[serde(rename = \"type\")]"));
        assert!(output.contains("pub type_: String,"));
    }

    #[test]
    fn test_toefl_all_models() {
        let sql = std::fs::read_to_string(
            "/home/abraham/Aplikasi-Ibrohim/new-toefl-quiz/src/db/migrations/001_initial_schema.sql"
        ).expect("read schema");
        let tables = schema_parser::parse_schema(&sql);

        println!("\n=== Generated models for {} tables ===\n", tables.len());
        for table in &tables {
            let output = generate_model_file(table);
            let struct_name = to_pascal_case(&table.name);

            // Verify basic structure
            assert!(output.contains(&format!("pub struct {} {{", struct_name)),
                "Missing struct for {}", table.name);
            assert!(output.contains(&format!("#[vil_entity(table = \"{}\")]", table.name)),
                "Missing vil_entity for {}", table.name);
            assert!(output.contains(&format!("pub struct Create{}Request", struct_name)),
                "Missing CreateRequest for {}", table.name);
            assert!(output.contains(&format!("pub struct Update{}Request", struct_name)),
                "Missing UpdateRequest for {}", table.name);

            // Count lines as rough size check
            let lines = output.lines().count();
            println!("  {} → {} ({} lines)", table.name, struct_name, lines);
        }
    }
}
