use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

pub fn create_project(name: &str, template: &str) -> Result<()> {
    // Try multiple paths to find templates
    let template_paths = [
        Path::new("crates/vil_cli/templates").join(template),
        Path::new("../vil_cli/templates").join(template),
        Path::new(".")
            .join("crates/vil_cli/templates")
            .join(template),
    ];

    let template_path = template_paths
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| anyhow!("Template '{}' not found in any location", template))?;

    if !template_path.exists() {
        return Err(anyhow!(
            "Template '{}' not found. Available: ai-inference, webhook-forwarder, event-fanout, stream-filter, load-balancer",
            template
        ));
    }

    // Create project directory
    fs::create_dir_all(name)?;

    // Copy template files
    copy_dir_all(template_path, Path::new(name))?;

    // Extract basename for package name (handle absolute paths)
    let project_name = Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name);

    // Resolve the crates path relative to the project location
    let project_abs = fs::canonicalize(name)?;
    let workspace_root = find_workspace_root().unwrap_or_else(|| std::env::current_dir().unwrap());
    let crates_dir = workspace_root.join("crates");

    let crates_path = if crates_dir.exists() {
        pathdiff_relative(&project_abs, &crates_dir)
    } else {
        "../crates".to_string()
    };

    // Replace placeholders in all files
    replace_in_dir(Path::new(name), &[
        ("{{PROJECT_NAME}}", project_name),
        ("{{VIL_CRATES}}", &crates_path),
    ])?;

    Ok(())
}

pub fn init_project(name: &str) -> Result<()> {
    let current_dir = std::env::current_dir()?;

    let src_dir = current_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let cargo_path = current_dir.join("Cargo.toml");
    if !cargo_path.exists() {
        let workspace_root = find_workspace_root().unwrap_or_else(|| current_dir.clone());
        let crates_dir = workspace_root.join("crates");
        let crates_path = if crates_dir.exists() {
            pathdiff_relative(&current_dir, &crates_dir)
        } else {
            "../crates".to_string()
        };

        let cargo_toml = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
vil_sdk = {{ path = "{}/vil_sdk" }}
serde_json = "1.0"

[workspace]
"#,
            name, crates_path
        );
        fs::write(&cargo_path, cargo_toml)?;
    }

    let main_rs = src_dir.join("main.rs");
    if !main_rs.exists() {
        let main_rs_content = r#"use std::sync::Arc;
use vil_sdk::prelude::*;

fn main() {
    println!("Hello from VIL!");

    // Initialize runtime
    let world = Arc::new(VastarRuntimeWorld::new_shared()
        .expect("Failed to initialize VIL SHM Runtime"));

    // TODO: Add your pipeline nodes here
    // See examples/001-vil-ai-gw-demo for a complete example.
}
"#;
        fs::write(&main_rs, main_rs_content)?;
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }

    Ok(())
}

fn replace_in_dir(dir: &Path, replacements: &[(&str, &str)]) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let path = entry.path();

        if ty.is_dir() {
            replace_in_dir(&path, replacements)?;
        } else if let Ok(content) = fs::read_to_string(&path) {
            let mut new_content = content.clone();
            for (from, to) in replacements {
                new_content = new_content.replace(from, to);
            }
            if new_content != content {
                fs::write(&path, new_content)?;
            }
        }
    }
    Ok(())
}

fn find_workspace_root() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("crates").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn pathdiff_relative(from: &Path, to: &Path) -> String {
    if let Ok(to_canon) = fs::canonicalize(to) {
        if let Ok(from_canon) = fs::canonicalize(from) {
            if let Some(rel) = pathdiff(&from_canon, &to_canon) {
                return rel.to_string_lossy().to_string();
            }
        }
    }
    to.to_string_lossy().to_string()
}

fn pathdiff(from: &Path, to: &Path) -> Option<std::path::PathBuf> {
    let mut from_parts: Vec<_> = from.components().collect();
    let mut to_parts: Vec<_> = to.components().collect();

    // Remove common prefix
    while !from_parts.is_empty() && !to_parts.is_empty() && from_parts[0] == to_parts[0] {
        from_parts.remove(0);
        to_parts.remove(0);
    }

    let mut result = std::path::PathBuf::new();
    for _ in &from_parts {
        result.push("..");
    }
    for part in &to_parts {
        result.push(part);
    }

    Some(result)
}
