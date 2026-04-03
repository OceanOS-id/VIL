// =============================================================================
// vil generate — Code generation for models, services, migrations, resources
// =============================================================================

use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run_generate(kind: &str, name: &str, fields: &[String]) -> Result<(), String> {
    match kind {
        "model" => generate_model(name, fields),
        "migration" => generate_migration(name, fields),
        "service" => generate_service(name),
        "resource" => {
            generate_model(name, fields)?;
            generate_migration(name, fields)?;
            generate_service(name)?;
            println!();
            println!(
                "  {} Register in main.rs:",
                "Tip:".yellow().bold()
            );
            println!(
                "    .service({}::crud_service(pool.clone()))",
                to_pascal(name)
            );
            Ok(())
        }
        _ => Err(format!(
            "Unknown generator '{}'. Use: model, service, migration, resource",
            kind
        )),
    }
}

fn generate_model(name: &str, fields: &[String]) -> Result<(), String> {
    let dir = Path::new("src/models");
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }

    let pascal = to_pascal(name);
    let snake = to_snake(name);
    let table = format!("{}s", &snake);

    let mut field_defs = String::new();
    for f in fields {
        let parts: Vec<&str> = f.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid field '{}'. Use name:type (e.g. username:string)", f));
        }
        let fname = parts[0];
        let ftype = map_type(parts[1]);
        field_defs.push_str(&format!("    pub {}: {},\n", fname, ftype));
    }

    let content = format!(
        r#"use serde::{{Deserialize, Serialize}};
use vil_orm::prelude::*;
use vil_server::prelude::VilModel;

#[derive(Debug, Clone, Serialize, Deserialize, VilModel, VilEntity, VilCrud, sqlx::FromRow)]
#[vil_entity(table = "{table}")]
#[vil_crud(prefix = "/api/{snake}")]
pub struct {pascal} {{
    #[vil_entity(pk, auto_uuid)]
    pub id: String,
{field_defs}    #[vil_entity(auto_now_add)]
    pub created_at: String,
    #[vil_entity(auto_now)]
    pub updated_at: String,
}}
"#
    );

    let path = dir.join(format!("{}.rs", &snake));
    fs::write(&path, content).map_err(|e| e.to_string())?;

    // Update mod.rs
    let mod_path = dir.join("mod.rs");
    let mod_line = format!("pub mod {};", &snake);
    if mod_path.exists() {
        let existing = fs::read_to_string(&mod_path).unwrap_or_default();
        if !existing.contains(&mod_line) {
            fs::write(&mod_path, format!("{}{}\n", existing, mod_line))
                .map_err(|e| e.to_string())?;
        }
    } else {
        fs::write(&mod_path, format!("{}\n", mod_line)).map_err(|e| e.to_string())?;
    }

    println!("  {} src/models/{}.rs", "Created".green().bold(), &snake);
    Ok(())
}

fn generate_migration(name: &str, fields: &[String]) -> Result<(), String> {
    let dir = Path::new("migrations");
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }

    // Find next version number
    let version = next_migration_version(dir);
    let snake = to_snake(name);
    let table = format!("{}s", &snake);

    // Build column SQL
    let mut cols = vec!["    id TEXT PRIMARY KEY".to_string()];
    for f in fields {
        let parts: Vec<&str> = f.split(':').collect();
        if parts.len() == 2 {
            let sql_type = map_sql_type(parts[1]);
            cols.push(format!("    {} {}", parts[0], sql_type));
        }
    }
    cols.push("    created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))".to_string());
    cols.push("    updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))".to_string());

    let up = format!(
        "CREATE TABLE IF NOT EXISTS {} (\n{}\n);\n",
        table,
        cols.join(",\n")
    );
    let down = format!("DROP TABLE IF EXISTS {};\n", table);

    let up_path = dir.join(format!("{:03}_{}.up.sql", version, &snake));
    let down_path = dir.join(format!("{:03}_{}.down.sql", version, &snake));

    fs::write(&up_path, up).map_err(|e| e.to_string())?;
    fs::write(&down_path, down).map_err(|e| e.to_string())?;

    println!(
        "  {} migrations/{:03}_{}.up.sql",
        "Created".green().bold(),
        version,
        &snake
    );
    println!(
        "  {} migrations/{:03}_{}.down.sql",
        "Created".green().bold(),
        version,
        &snake
    );
    Ok(())
}

fn generate_service(name: &str) -> Result<(), String> {
    let dir = Path::new("src/services");
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }

    let pascal = to_pascal(name);
    let snake = to_snake(name);

    let content = format!(
        r#"use vil::prelude::*;
use crate::models::{snake}::{pascal};

// {pascal} uses VilCrud — auto CRUD service.
// Register in main.rs:
//   .service({pascal}::crud_service(pool.clone()))
//
// To add custom endpoints beyond CRUD:
// #[vil_handler]
// pub async fn custom_action(ctx: ServiceCtx) -> VilResult<String> {{
//     Ok(VilResponse::ok("custom"))
// }}
"#
    );

    let path = dir.join(format!("{}.rs", &snake));
    fs::write(&path, content).map_err(|e| e.to_string())?;

    // Update mod.rs
    let mod_path = dir.join("mod.rs");
    let mod_line = format!("pub mod {};", &snake);
    if mod_path.exists() {
        let existing = fs::read_to_string(&mod_path).unwrap_or_default();
        if !existing.contains(&mod_line) {
            fs::write(&mod_path, format!("{}{}\n", existing, mod_line))
                .map_err(|e| e.to_string())?;
        }
    } else {
        fs::write(&mod_path, format!("{}\n", mod_line)).map_err(|e| e.to_string())?;
    }

    println!("  {} src/services/{}.rs", "Created".green().bold(), &snake);
    Ok(())
}

// ── Helpers ──

fn to_pascal(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

fn to_snake(s: &str) -> String {
    s.to_lowercase().replace('-', "_")
}

fn map_type(t: &str) -> &str {
    match t {
        "string" | "text" | "uuid" | "datetime" => "String",
        "integer" | "int" | "i64" => "i64",
        "float" | "f64" | "real" => "f64",
        "boolean" | "bool" => "i32",
        "json" => "Option<String>",
        _ => "String",
    }
}

fn map_sql_type(t: &str) -> &str {
    match t {
        "string" | "text" | "uuid" | "datetime" => "TEXT",
        "integer" | "int" | "i64" => "INTEGER",
        "float" | "f64" | "real" => "REAL",
        "boolean" | "bool" => "INTEGER",
        "json" => "TEXT",
        _ => "TEXT",
    }
}

fn next_migration_version(dir: &Path) -> u32 {
    let mut max = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(v) = name.split('_').next() {
                if let Ok(n) = v.parse::<u32>() {
                    max = max.max(n);
                }
            }
        }
    }
    max + 1
}
