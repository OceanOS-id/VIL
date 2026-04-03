//! vil orm — CLI command handler for VilORM project generation.

use colored::Colorize;
use std::path::Path;
use vil_cli_server::orm::{model_gen, project_gen, schema_parser, service_gen};

/// Run `vil orm gen <target> --schema <file> [--output <dir>] [--name <name>] [--table <table>]`
pub fn run_orm_gen(
    target: &str,
    schema_path: &str,
    output: Option<&str>,
    name: Option<&str>,
    table: Option<&str>,
) -> Result<(), String> {
    println!();
    println!("  {}", "╔══════════════════════════════════════════════════╗".cyan());
    println!("  {}  {} — VilORM Project Generator             {}", "║".cyan(), "vil orm".green().bold(), "║".cyan());
    println!("  {}", "╚══════════════════════════════════════════════════╝".cyan());
    println!();

    // Read schema
    let sql = std::fs::read_to_string(schema_path)
        .map_err(|e| format!("Failed to read schema '{}': {}", schema_path, e))?;

    // Parse
    let tables = schema_parser::parse_schema(&sql);
    if tables.is_empty() {
        return Err("No CREATE TABLE statements found in schema file".into());
    }
    println!("  {}  Parsed {} tables from {}", "✓".green(), tables.len(), schema_path);

    match target {
        "all" => gen_all(&tables, &sql, output, name)?,
        "model" => gen_single_model(&tables, table)?,
        "service" => gen_single_service(&tables, table)?,
        other => return Err(format!("Unknown target '{}'. Use: all, model, service", other)),
    }

    Ok(())
}

/// Generate complete project
fn gen_all(
    tables: &[schema_parser::TableMeta],
    schema_sql: &str,
    output: Option<&str>,
    name: Option<&str>,
) -> Result<(), String> {
    let output_dir = Path::new(output.unwrap_or("."));
    let project_name = name.unwrap_or_else(|| {
        output_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("vil-app")
    });

    println!("  {}  Project: {}", "→".dimmed(), project_name.cyan());
    println!("  {}  Output:  {}", "→".dimmed(), output_dir.display());
    println!();

    let files = project_gen::generate_project(output_dir, project_name, tables, schema_sql)?;

    // Summary
    let model_count = files.keys().filter(|k| k.starts_with("src/models/") && *k != "src/models/mod.rs").count();
    let svc_count = files.keys().filter(|k| k.starts_with("src/services/") && *k != "src/services/mod.rs").count();
    let endpoints = svc_count * 5;

    println!("  {}", "Generated:".green().bold());
    println!("    {} model files", model_count);
    println!("    {} service files ({} endpoints)", svc_count, endpoints);
    println!("    Cargo.toml + main.rs + db.rs + error.rs");
    println!();
    println!("  {}", "Next steps:".yellow());
    println!("    cd {}", output_dir.display());
    println!("    cargo build");
    println!("    cargo run");
    println!("    # API at http://localhost:8080/api/<table>/");
    println!();

    // List tables
    println!("  {}", "Tables:".dimmed());
    for t in tables {
        let struct_name = model_gen::to_pascal_case(&t.name);
        println!("    {} → /api/{}/  ({})", struct_name, t.name,
            format!("{} cols, pk={}", t.columns.len(), t.primary_key).dimmed());
    }
    println!();

    Ok(())
}

/// Generate single model to stdout
fn gen_single_model(
    tables: &[schema_parser::TableMeta],
    table_name: Option<&str>,
) -> Result<(), String> {
    let name = table_name.ok_or("--table required for 'model' target")?;
    let table = tables.iter().find(|t| t.name == name)
        .ok_or_else(|| format!("Table '{}' not found in schema", name))?;

    let output = model_gen::generate_model_file(table);
    println!("{}", output);
    Ok(())
}

/// Generate single service to stdout
fn gen_single_service(
    tables: &[schema_parser::TableMeta],
    table_name: Option<&str>,
) -> Result<(), String> {
    let name = table_name.ok_or("--table required for 'service' target")?;
    let table = tables.iter().find(|t| t.name == name)
        .ok_or_else(|| format!("Table '{}' not found in schema", name))?;

    let output = service_gen::generate_service_file(table);
    println!("{}", output);
    Ok(())
}
