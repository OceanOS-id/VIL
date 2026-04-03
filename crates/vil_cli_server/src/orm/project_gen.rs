//! Project Generator — Generates a complete runnable VilApp project from schema.
//!
//! Output files:
//! - Cargo.toml
//! - src/main.rs (VilApp + ServiceProcess wiring)
//! - src/db.rs (SqlxPool connect + migration)
//! - src/error.rs (AppError + VilError bridge)
//! - src/models/mod.rs + per-table model files
//! - src/services/mod.rs + per-table service files

use super::model_gen::generate_model_file;
use super::schema_parser::TableMeta;
use super::service_gen::generate_service_file;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Generate a complete project at the given directory.
/// `schema_sql` is the raw SQL DDL content (for embedding in db.rs).
pub fn generate_project(
    output_dir: &Path,
    project_name: &str,
    tables: &[TableMeta],
    schema_sql: &str,
) -> Result<HashMap<String, String>, String> {
    let mut files: HashMap<String, String> = HashMap::new();

    files.insert("Cargo.toml".into(), gen_cargo_toml(project_name));
    files.insert("src/main.rs".into(), gen_main_rs(project_name, tables));
    files.insert("src/db.rs".into(), gen_db_rs(schema_sql));
    files.insert("src/error.rs".into(), gen_error_rs());
    files.insert("schema/001_initial.sql".into(), schema_sql.to_string());

    // Models
    let mut model_mods = Vec::new();
    for table in tables {
        let filename = format!("src/models/{}.rs", table.name);
        files.insert(filename, generate_model_file(table));
        model_mods.push(table.name.clone());
    }
    files.insert("src/models/mod.rs".into(), gen_mod_rs(&model_mods));

    // Services
    let mut service_mods = Vec::new();
    for table in tables {
        let filename = format!("src/services/{}_svc.rs", table.name);
        files.insert(filename, generate_service_file(table));
        service_mods.push(format!("{}_svc", table.name));
    }
    files.insert("src/services/mod.rs".into(), gen_mod_rs(&service_mods));

    // Write files to disk
    for (path, content) in &files {
        let full_path = output_dir.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
        }
        fs::write(&full_path, content)
            .map_err(|e| format!("write {}: {}", full_path.display(), e))?;
    }

    Ok(files)
}

/// Generate Cargo.toml
fn gen_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
# VIL Framework
vil_server = "0.1"
vil_server_core = "0.1"
vil_server_auth = "0.1"
vil_json = "0.1"
vil_db_sqlx = {{ version = "0.1", features = ["sqlite"] }}
vil_orm = "0.1"
vil_orm_derive = "0.1"
vil_log = "0.1"
bytes = "1"

# Database
sqlx = {{ version = "0.8", features = ["runtime-tokio", "sqlite", "any"] }}

# Async
tokio = {{ version = "1", features = ["full"] }}

# Serialization
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"

# Utilities
uuid = {{ version = "1", features = ["v4"] }}
chrono = {{ version = "0.4", features = ["serde"] }}

[dev-dependencies]
reqwest = {{ version = "0.12", features = ["json"] }}
"#,
        name = name,
    )
}

/// Generate src/main.rs with VilApp + ServiceProcess per table
fn gen_main_rs(project_name: &str, tables: &[TableMeta]) -> String {
    let mut out = String::with_capacity(4096);

    // Imports
    out.push_str("use std::sync::Arc;\n");
    out.push_str("use vil_server::prelude::*;\n\n");
    out.push_str("mod db;\n");
    out.push_str("mod error;\n");
    out.push_str("mod models;\n");
    out.push_str("mod services;\n\n");

    // Service imports
    out.push_str("use services::{");
    let svc_imports: Vec<String> = tables.iter().map(|t| format!("{}_svc", t.name)).collect();
    out.push_str(&svc_imports.join(", "));
    out.push_str("};\n\n");

    // AppState
    out.push_str(
        "#[derive(Clone)]\n\
         pub struct AppState {\n\
         \x20   pub pool: Arc<vil_db_sqlx::SqlxPool>,\n\
         }\n\n",
    );

    // main
    out.push_str("#[tokio::main]\n");
    out.push_str("async fn main() {\n");
    out.push_str("    let _log = vil_log::init()\n");
    out.push_str("        .dev_mode(cfg!(debug_assertions))\n");
    out.push_str("        .build();\n\n");
    out.push_str("    let pool = db::connect().await;\n");
    out.push_str("    let state = AppState { pool: Arc::new(pool) };\n\n");

    // ServiceProcess per table
    for table in tables {
        let svc_var = format!("{}_svc", table.name);
        let svc_mod = format!("{}_svc", table.name);

        if table.is_composite_pk() {
            // Composite PK routes: /:col1/:col2/...
            let pk_path: String = table
                .primary_keys
                .iter()
                .map(|pk| format!("/:{}", pk))
                .collect();

            out.push_str(&format!(
                "    let {var} = ServiceProcess::new(\"{name}\")\n\
                 \x20       .endpoint(Method::GET, \"/list\", get({mod}::list))\n\
                 \x20       .endpoint(Method::GET, \"{pk_path}\", get({mod}::get_by_pk))\n\
                 \x20       .endpoint(Method::POST, \"/create\", post({mod}::create))\n\
                 \x20       .endpoint(Method::PUT, \"{pk_path}\", put({mod}::update))\n\
                 \x20       .endpoint(Method::DELETE, \"{pk_path}\", delete({mod}::delete))\n\
                 \x20       .state(state.clone());\n\n",
                var = svc_var,
                name = table.name,
                mod = svc_mod,
                pk_path = pk_path,
            ));
        } else {
            out.push_str(&format!(
                "    let {var} = ServiceProcess::new(\"{name}\")\n\
                 \x20       .endpoint(Method::GET, \"/list\", get({mod}::list))\n\
                 \x20       .endpoint(Method::GET, \"/:id\", get({mod}::get_by_id))\n\
                 \x20       .endpoint(Method::POST, \"/create\", post({mod}::create))\n\
                 \x20       .endpoint(Method::PUT, \"/:id\", put({mod}::update))\n\
                 \x20       .endpoint(Method::DELETE, \"/:id\", delete({mod}::delete))\n\
                 \x20       .state(state.clone());\n\n",
                var = svc_var,
                name = table.name,
                mod = svc_mod,
            ));
        }
    }

    // VilApp
    let port = 8080;
    out.push_str(&format!(
        "    let app = VilApp::new(\"{}\")\n\
         \x20       .port({})\n\
         \x20       .observer(true)\n",
        project_name, port,
    ));
    for table in tables {
        out.push_str(&format!("        .service({}_svc)\n", table.name));
    }
    out.push_str(";\n\n");
    out.push_str(&format!("    println!(\"[server] Starting on port {}\");\n", port));
    out.push_str("    app.run().await;\n");
    out.push_str("}\n");

    out
}

/// Generate src/db.rs — reads schema from file at runtime.
fn gen_db_rs(_schema_sql: &str) -> String {
    let mut out = String::new();
    out.push_str("use vil_db_sqlx::{SqlxConfig, SqlxPool};\n\n");
    out.push_str("pub async fn connect() -> SqlxPool {\n");
    out.push_str("    let url = std::env::var(\"DATABASE_URL\")\n");
    out.push_str("        .unwrap_or_else(|_| \"sqlite:data.db?mode=rwc\".into());\n\n");
    out.push_str("    let pool = SqlxPool::connect(\"app\", SqlxConfig::sqlite(&url))\n");
    out.push_str("        .await\n");
    out.push_str("        .expect(\"Database connection failed\");\n\n");
    out.push_str("    // Run schema from file\n");
    out.push_str("    let schema_path = std::env::var(\"SCHEMA_PATH\")\n");
    out.push_str("        .unwrap_or_else(|_| \"schema/001_initial.sql\".into());\n");
    out.push_str("    if let Ok(sql) = std::fs::read_to_string(&schema_path) {\n");
    out.push_str("        // Split by CREATE TABLE and execute each\n");
    out.push_str("        let mut buf = String::new();\n");
    out.push_str("        for line in sql.lines() {\n");
    out.push_str("            let trimmed = line.trim();\n");
    out.push_str("            if trimmed.starts_with(\"--\") || trimmed.is_empty() { continue; }\n");
    out.push_str("            buf.push_str(trimmed);\n");
    out.push_str("            buf.push(' ');\n");
    out.push_str("            if trimmed.ends_with(\");\") {\n");
    out.push_str("                if let Err(e) = pool.execute_raw(&buf).await {\n");
    out.push_str("                    eprintln!(\"[db] Warning: {e}\");\n");
    out.push_str("                }\n");
    out.push_str("                buf.clear();\n");
    out.push_str("            }\n");
    out.push_str("        }\n");
    out.push_str("        println!(\"[db] Schema applied from {}\", schema_path);\n");
    out.push_str("    } else {\n");
    out.push_str("        println!(\"[db] No schema file at {} — skipping migration\", schema_path);\n");
    out.push_str("    }\n\n");
    out.push_str("    pool\n");
    out.push_str("}\n");
    out
}

/// Generate src/error.rs
fn gen_error_rs() -> String {
    r#"use serde::Serialize;
use vil_server::axum::http::StatusCode;
use vil_server::axum::response::{IntoResponse, Json, Response};

#[derive(Debug)]
pub enum AppError {
    Validation(String),
    NotFound(String),
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    ok: bool,
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            Self::Validation(m) => (StatusCode::UNPROCESSABLE_ENTITY, m.clone()),
            Self::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into()),
        };
        (status, Json(ErrorBody { ok: false, error: msg })).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        vil_log::db_log!(Error, vil_log::DbPayload {
            error_code: 1,
            ..vil_log::DbPayload::default()
        });
        Self::Internal(e.to_string())
    }
}

impl From<AppError> for vil_server::prelude::VilError {
    fn from(e: AppError) -> Self {
        match e {
            AppError::Validation(m) => vil_server::prelude::VilError::validation(m),
            AppError::NotFound(m) => vil_server::prelude::VilError::not_found(m),
            AppError::Internal(_) => vil_server::prelude::VilError::internal("Internal error"),
        }
    }
}
"#
    .to_string()
}

/// Generate a mod.rs that declares all sub-modules
fn gen_mod_rs(modules: &[String]) -> String {
    let mut out = String::new();
    for m in modules {
        out.push_str(&format!("#[allow(dead_code)]\npub mod {};\n", m));
    }
    out
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orm::schema_parser;

    #[test]
    fn test_cargo_toml() {
        let toml = gen_cargo_toml("my-app");
        assert!(toml.contains("name = \"my-app\""));
        assert!(toml.contains("vil_orm = \"0.1\""));
        assert!(toml.contains("vil_server = \"0.1\""));
        assert!(toml.contains("sqlx"));
        assert!(toml.contains("uuid"));
    }

    #[test]
    fn test_error_rs() {
        let err = gen_error_rs();
        assert!(err.contains("pub enum AppError"));
        assert!(err.contains("impl IntoResponse for AppError"));
        assert!(err.contains("impl From<sqlx::Error> for AppError"));
        assert!(err.contains("impl From<AppError> for vil_server::prelude::VilError"));
    }

    #[test]
    fn test_main_rs_structure() {
        let sql = r#"
CREATE TABLE profiles (id TEXT PRIMARY KEY, username TEXT, created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')));
CREATE TABLE posts (id TEXT PRIMARY KEY, title TEXT NOT NULL, created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')));
        "#;
        let tables = schema_parser::parse_schema(sql);
        let main = gen_main_rs("test-app", &tables);

        assert!(main.contains("VilApp::new(\"test-app\")"));
        assert!(main.contains("mod models;"));
        assert!(main.contains("mod services;"));
        assert!(main.contains("use services::{profiles_svc, posts_svc}"));
        assert!(main.contains("ServiceProcess::new(\"profiles\")"));
        assert!(main.contains("ServiceProcess::new(\"posts\")"));
        assert!(main.contains(".service(profiles_svc)"));
        assert!(main.contains(".service(posts_svc)"));
        assert!(main.contains("profiles_svc::list"));
        assert!(main.contains("profiles_svc::create"));
        assert!(main.contains("posts_svc::get_by_id"));
        assert!(main.contains("db::connect()"));
        assert!(main.contains("vil_log::init()"));
    }

    #[test]
    fn test_db_rs_reads_schema_file() {
        let schema = "CREATE TABLE test (id TEXT PRIMARY KEY);";
        let db = gen_db_rs(schema);
        assert!(db.contains("SqlxPool::connect"));
        assert!(db.contains("execute_raw"));
        assert!(db.contains("SCHEMA_PATH"));
        assert!(db.contains("read_to_string"));
    }

    #[test]
    fn test_generate_full_project() {
        let sql = std::fs::read_to_string(
            "/home/abraham/Aplikasi-Ibrohim/new-toefl-quiz/src/db/migrations/001_initial_schema.sql"
        ).expect("read schema");
        let tables = schema_parser::parse_schema(&sql);

        let dir = std::env::temp_dir().join("vil-orm-test-project");
        let _ = fs::remove_dir_all(&dir);

        let files = generate_project(&dir, "toefl-quiz-gen", &tables, &sql).expect("generate");

        println!("\n=== Generated project: {} files ===", files.len());

        // Verify key files exist
        assert!(dir.join("Cargo.toml").exists(), "Missing Cargo.toml");
        assert!(dir.join("src/main.rs").exists(), "Missing main.rs");
        assert!(dir.join("src/db.rs").exists(), "Missing db.rs");
        assert!(dir.join("src/error.rs").exists(), "Missing error.rs");
        assert!(dir.join("src/models/mod.rs").exists(), "Missing models/mod.rs");
        assert!(dir.join("src/services/mod.rs").exists(), "Missing services/mod.rs");

        // Verify one model + service per table
        for table in &tables {
            let model_path = dir.join(format!("src/models/{}.rs", table.name));
            assert!(model_path.exists(), "Missing model: {}", table.name);
            let svc_path = dir.join(format!("src/services/{}_svc.rs", table.name));
            assert!(svc_path.exists(), "Missing service: {}", table.name);
        }

        // Count
        let model_count = fs::read_dir(dir.join("src/models")).unwrap().count() - 1; // minus mod.rs
        let svc_count = fs::read_dir(dir.join("src/services")).unwrap().count() - 1;
        println!("  Models: {}, Services: {}", model_count, svc_count);
        assert_eq!(model_count, tables.len());
        assert_eq!(svc_count, tables.len());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }
}
