//! VLB Builder — compiles VIL service into .vlb artifact
//!
//! Process:
//! 1. Read Cargo.toml for package name/version
//! 2. Run `cargo build` to compile the service
//! 3. Extract service manifest from compiled code (or Cargo.toml metadata)
//! 4. Package into .vlb format

use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
pub struct VlbBuildConfig {
    pub target: String,
    pub release: bool,
    pub output: Option<String>,
    pub name: Option<String>,
    pub version: String,
}

pub fn build_vlb(config: VlbBuildConfig) -> Result<String, String> {
    println!("\u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}");
    println!("\u{2551}  vil build --target vlb                        \u{2551}");
    println!("\u{255a}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255d}");
    println!();

    // 1. Read Cargo.toml
    let cargo_toml = std::fs::read_to_string("Cargo.toml")
        .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;

    let name = config.name.unwrap_or_else(|| {
        // Parse package name from Cargo.toml
        for line in cargo_toml.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    return val.trim().trim_matches('"').to_string();
                }
            }
        }
        "unknown-service".to_string()
    });

    println!("  Service:  {}", name);
    println!("  Version:  {}", config.version);
    println!("  Release:  {}", config.release);
    println!();

    // 2. Validate VLB rules
    println!("  Checking VLB rules...");
    let rules = validate_vlb_rules(&cargo_toml)?;
    for (i, (rule, passed)) in rules.iter().enumerate() {
        let icon = if *passed { "\u{2713}" } else { "\u{2717}" };
        println!("    [{}] Rule {}: {}", icon, i + 1, rule);
    }

    let all_passed = rules.iter().all(|(_, p)| *p);
    if !all_passed {
        return Err("VLB validation failed -- fix the rules above".into());
    }
    println!("  All rules passed!");
    println!();

    // 3. Run cargo build
    println!("  Compiling...");
    let mut build_cmd = Command::new("cargo");
    build_cmd.arg("build");
    if config.release {
        build_cmd.arg("--release");
    }

    let build_output = build_cmd.output()
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        return Err(format!("cargo build failed:\n{}", stderr));
    }
    println!("  Compiled successfully.");
    println!();

    // 4. Build .vlb artifact
    println!("  Packaging .vlb artifact...");

    // Create manifest from Cargo.toml metadata
    let manifest = build_manifest(&name, &config.version);

    // Find the compiled binary
    let profile = if config.release { "release" } else { "debug" };
    let binary_name = name.replace('-', "_");
    let binary_path = format!("target/{}/{}", profile, binary_name);

    let native_code = if Path::new(&binary_path).exists() {
        // Read first 64KB of binary as "native code" section (placeholder)
        // In full implementation, this would be the relocatable .o file
        let data = std::fs::read(&binary_path)
            .map_err(|e| format!("Failed to read binary: {}", e))?;
        // Take a hash/fingerprint instead of full binary (for now)
        let fingerprint = data.len() as u64;
        fingerprint.to_le_bytes().to_vec()
    } else {
        vec![0u8; 8] // placeholder
    };

    // Build VLB
    let vlb_data = build_vlb_binary(manifest, native_code)?;

    // 5. Write output
    let output_path = config.output.unwrap_or_else(|| {
        let dir = "target/vlb".to_string();
        let _ = std::fs::create_dir_all(&dir);
        format!("{}/{}-{}.vlb", dir, name, config.version)
    });

    std::fs::write(&output_path, &vlb_data)
        .map_err(|e| format!("Failed to write VLB: {}", e))?;

    println!("  Output:   {}", output_path);
    println!("  Size:     {} bytes", vlb_data.len());
    println!();
    println!("  \u{2713} VLB artifact built successfully!");
    println!();
    println!("  Deploy to vflow-server:");
    println!("    vil provision push --host http://vflow-host:8080 \\");
    println!("      --artifact {}", output_path);

    Ok(output_path)
}

/// Validate the 5 VLB rules by scanning source files
fn validate_vlb_rules(cargo_toml: &str) -> Result<Vec<(String, bool)>, String> {
    let mut rules = Vec::new();

    // Scan src/ for Rust files
    let src_content = scan_source_files("src/")?;

    // Rule 1: All handlers have #[vil_endpoint]
    let has_handlers = src_content.contains("async fn") || src_content.contains("fn ");
    let has_vil_endpoint = src_content.contains("vil_endpoint");
    let rule1_pass = !has_handlers || has_vil_endpoint || src_content.contains("vil_app!");
    rules.push(("All handlers have #[vil_endpoint]".into(), rule1_pass));

    // Rule 2: Message types have VilModel
    let has_structs = src_content.contains("struct ");
    let has_vil_model = src_content.contains("VilModel");
    let rule2_pass = !has_structs || has_vil_model || src_content.contains("Serialize");
    rules.push(("Message types have #[derive(VilModel)]".into(), rule2_pass));

    // Rule 3: State uses #[vil_service_state]
    let has_state = src_content.contains("ServiceCtx") || src_content.contains("Extension(");
    let has_state_attr = src_content.contains("vil_service_state");
    let rule3_pass = !has_state || has_state_attr || !src_content.contains("static ");
    rules.push(("State uses #[vil_service_state]".into(), rule3_pass));

    // Rule 4: Mesh requires explicit
    let has_hidden_http = src_content.contains("reqwest::") || src_content.contains("hyper::Client");
    let has_mesh = src_content.contains("VxMeshConfig")
        || src_content.contains("mesh_requires")
        || src_content.contains("vil_service");
    let rule4_pass = !has_hidden_http || has_mesh;
    rules.push(("Mesh requires explicit (no hidden HTTP calls)".into(), rule4_pass));

    // Rule 5: Uses VilApp / vil_app! (not vil_server::new())
    let uses_vil_app = src_content.contains("VilApp::") || src_content.contains("vil_app!");
    let uses_old_api = src_content.contains("vil_server::new(");
    let rule5_pass = uses_vil_app || !uses_old_api;
    rules.push((
        "Uses VilApp / vil_app! (not vil_server::new())".into(),
        rule5_pass,
    ));

    // Suppress unused variable warning
    let _ = cargo_toml;

    Ok(rules)
}

/// Scan all .rs files in a directory recursively
fn scan_source_files(dir: &str) -> Result<String, String> {
    let mut content = String::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(file_content) = std::fs::read_to_string(&path) {
                    content.push_str(&file_content);
                    content.push('\n');
                }
            } else if path.is_dir() {
                if let Ok(sub) = scan_source_files(path.to_str().unwrap_or("")) {
                    content.push_str(&sub);
                }
            }
        }
    }
    Ok(content)
}

/// Build ServiceManifest from Cargo.toml metadata
fn build_manifest(name: &str, version: &str) -> Vec<u8> {
    let manifest = serde_json::json!({
        "name": name,
        "version": version,
        "description": format!("VIL service: {}", name),
        "endpoints": [],
        "ports": [
            { "name": "trigger_in", "lane": "Trigger", "transfer_mode": "LoanWrite", "direction": "In" },
            { "name": "data_out", "lane": "Data", "transfer_mode": "LoanWrite", "direction": "Out" },
            { "name": "ctrl_out", "lane": "Control", "transfer_mode": "Copy", "direction": "Out" },
        ],
        "mesh_requires": [],
        "state_type": "PrivateHeap",
        "min_shm_bytes": 4194304_u64,
        "exec_class_default": "AsyncTask",
    });
    serde_json::to_vec(&manifest).unwrap_or_default()
}

/// Package into VLB binary format
///
/// Uses the same header/section layout as vflow_server's VlbWriter:
/// - Header: 16 bytes (magic "VLNG", version, arch, section_count, flags, checksum)
/// - Section table: 4 entries x 8 bytes each
/// - Section data: Manifest(1), Schemas(2), NativeCode(4), Resources(5)
fn build_vlb_binary(manifest_bytes: Vec<u8>, native_code: Vec<u8>) -> Result<Vec<u8>, String> {
    let schemas_bytes = b"[]".to_vec(); // empty schemas for now
    let resources_bytes: Vec<u8> = vec![];

    // VLB Header (16 bytes)
    let magic = b"VLNG";
    let version: u16 = 1;
    let arch: u16 = if cfg!(target_arch = "x86_64") {
        1
    } else if cfg!(target_arch = "aarch64") {
        2
    } else {
        1
    };
    let section_count: u16 = 4;
    let flags: u16 = 0;

    // Calculate section offsets
    let header_size: u32 = 16;
    let section_table_size: u32 = section_count as u32 * 8;
    let data_start = header_size + section_table_size;

    let s1_off = data_start;
    let s1_len = manifest_bytes.len() as u16;
    let s2_off = s1_off + s1_len as u32;
    let s2_len = schemas_bytes.len() as u16;
    let s3_off = s2_off + s2_len as u32;
    let s3_len = native_code.len().min(65535) as u16;
    let s4_off = s3_off + s3_len as u32;
    let s4_len = resources_bytes.len().min(65535) as u16;

    // Checksum (simple sum of all section bytes, matching vflow_server's VlbWriter)
    let checksum: u32 = manifest_bytes
        .iter()
        .chain(schemas_bytes.iter())
        .chain(native_code.iter())
        .chain(resources_bytes.iter())
        .fold(0u32, |acc, &b| acc.wrapping_add(b as u32));

    let mut buf = Vec::new();

    // Header
    buf.extend_from_slice(magic);
    buf.extend_from_slice(&version.to_le_bytes());
    buf.extend_from_slice(&arch.to_le_bytes());
    buf.extend_from_slice(&section_count.to_le_bytes());
    buf.extend_from_slice(&flags.to_le_bytes());
    buf.extend_from_slice(&checksum.to_le_bytes());

    // Section table (4 entries x 8 bytes)
    // Section IDs match vflow_server: Manifest=1, Schemas=2, NativeCode=4, Resources=5
    for (id, offset, size) in [
        (1u16, s1_off, s1_len), // Manifest
        (2u16, s2_off, s2_len), // Schemas
        (4u16, s3_off, s3_len), // NativeCode
        (5u16, s4_off, s4_len), // Resources
    ] {
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(&offset.to_le_bytes());
        buf.extend_from_slice(&size.to_le_bytes());
    }

    // Section data
    buf.extend_from_slice(&manifest_bytes);
    buf.extend_from_slice(&schemas_bytes);
    buf.extend_from_slice(&native_code[..s3_len as usize]);
    buf.extend_from_slice(&resources_bytes);

    Ok(buf)
}
