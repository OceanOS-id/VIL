// =============================================================================
// VIL Transpile SDK — Compiler
// =============================================================================
// Implements `vil compile` command:
//   --from yaml|python|typescript|go|java
//   --input <file>
//   --output <binary-name>
//   --release
//
// Flow:
//   1. Obtain manifest YAML (direct file or run source with VIL_COMPILE_MODE)
//   2. Parse YAML -> WorkflowManifest
//   3. Validate
//   4. Generate Rust code + Cargo.toml
//   5. Write to /tmp/vil-compile-<name>/
//   6. cargo build [--release]
//   7. Copy binary to output path
// =============================================================================

use crate::codegen;
use vil_cli_core::manifest::WorkflowManifest;
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Supported source languages for `--from`.
const SUPPORTED_LANGS: &[&str] = &[
    "yaml",
    "python",
    "typescript",
    "go",
    "java",
    "csharp",
    "kotlin",
    "swift",
    "zig",
];

/// Configuration for the compile command.
pub struct CompileConfig {
    pub from: String,
    pub input: String,
    pub output: Option<String>,
    pub release: bool,
    /// When true, also produce a .vlb artifact alongside the binary.
    pub target_vlb: bool,
    /// When true, save the generated YAML manifest in the same folder as the input file.
    pub save_manifest: bool,
}

/// Run the full compile pipeline.
pub fn run_compile(config: CompileConfig) -> Result<(), String> {
    // Validate --from
    let from = config.from.to_lowercase();
    if !SUPPORTED_LANGS.contains(&from.as_str()) {
        return Err(format!(
            "Unsupported source language '{}'. Supported: {:?}",
            config.from, SUPPORTED_LANGS
        ));
    }

    println!(
        "{} vil compile (from={}, input={})",
        ">>>".cyan().bold(),
        from,
        config.input
    );

    // Step 1: Obtain manifest YAML
    let yaml_content = obtain_manifest_yaml(&from, &config.input)?;

    // Step 2: Parse YAML -> WorkflowManifest
    println!("  {} Parsing manifest...", "[2/7]".dimmed());
    let manifest = WorkflowManifest::from_yaml(&yaml_content)?;

    // Step 3: Validate
    println!("  {} Validating manifest...", "[3/7]".dimmed());
    let mode = manifest.manifest_mode();
    if let Err(errors) = manifest.validate() {
        return Err(format!(
            "Manifest validation failed:\n  - {}",
            errors.join("\n  - ")
        ));
    }

    match mode {
        "workflow" => {
            println!(
                "  {} {} '{}' workflow mode ({} nodes, {} routes)",
                "OK".green().bold(),
                manifest.vil_version,
                manifest.name,
                manifest.nodes.len(),
                manifest.workflow_routes.len(),
            );
        }
        _ => {
            println!(
                "  {} {} '{}' with {} endpoint(s)",
                "OK".green().bold(),
                manifest.vil_version,
                manifest.name,
                manifest.endpoints.len()
            );
        }
    }

    // Save manifest YAML if requested
    if config.save_manifest && from != "yaml" {
        let input_path = Path::new(&config.input);
        let manifest_path = input_path.with_extension("vil.yaml");
        std::fs::write(&manifest_path, &yaml_content)
            .map_err(|e| format!("Failed to save manifest: {}", e))?;
        println!(
            "  {} Manifest saved: {}",
            "OK".green().bold(),
            manifest_path.display()
        );
    }

    // Step 4: Generate Rust code + Cargo.toml
    println!("  {} Generating Rust source...", "[4/7]".dimmed());

    // Detect SDK vs source mode
    // crate_prefix: the directory containing vil_sdk/, vil_rt/, etc.
    // SDK mode: ~/.vil/sdk/current/internal  (flat: internal/vil_sdk)
    // Source mode: {workspace}/crates           (nested: crates/vil_sdk)
    let crate_prefix = if vil_cli_core::sdk_path::is_sdk_installed() {
        let sdk_path = vil_cli_core::sdk_path::sdk_current_path()
            .join("internal")
            .to_string_lossy()
            .to_string();
        println!("  {} Using pre-compiled SDK at {}", "SDK".cyan(), sdk_path);
        sdk_path
    } else {
        let root = find_workspace_root()?;
        let prefix = format!("{}/crates", root);
        println!("  {} Using source crates at {}", "SRC".dimmed(), prefix);
        prefix
    };
    let (rust_source, cargo_toml) = match mode {
        "workflow" => (
            codegen::generate_workflow_rust(&manifest),
            codegen::generate_workflow_cargo_toml(&manifest, &crate_prefix),
        ),
        _ => (
            codegen::generate_rust(&manifest),
            codegen::generate_cargo_toml(&manifest, &crate_prefix),
        ),
    };

    // Step 5: Write to /tmp/vil-compile-<name>/
    println!("  {} Writing build directory...", "[5/7]".dimmed());
    let build_dir = write_build_dir(&manifest.name, &rust_source, &cargo_toml)?;
    println!("    {}", build_dir.display());

    // Step 6: cargo build
    println!("  {} Building with cargo...", "[6/7]".dimmed());
    cargo_build(&build_dir, config.release)?;

    // Step 7: Copy binary to output path
    println!("  {} Copying binary...", "[7/7]".dimmed());
    let output_name = config.output.unwrap_or_else(|| manifest.name.clone());
    let binary = copy_binary(&build_dir, &manifest.name, &output_name, config.release)?;

    println!("\n{} Compiled: {}", "OK".green().bold(), binary.display());

    // Show run instruction
    println!();
    println!("  {}", "Run:".green().bold());
    println!("    {} &", binary.display());
    println!();

    // Show usage hints
    if manifest.is_workflow() {
        // Find first sink port for curl example
        let sink_port = manifest
            .nodes
            .values()
            .find(|n| n.node_type == "http-sink")
            .and_then(|n| n.port)
            .unwrap_or(manifest.port);
        let sink_path = manifest
            .nodes
            .values()
            .find(|n| n.node_type == "http-sink")
            .and_then(|n| n.path.as_deref())
            .unwrap_or("/trigger");
        let has_sse = manifest
            .nodes
            .values()
            .any(|n| n.format.as_deref() == Some("sse"));

        println!();
        println!("  {}", "Test:".cyan());
        if has_sse {
            println!(
                "    curl -N -X POST http://localhost:{}{} \\",
                sink_port, sink_path
            );
            println!("      -H \"Content-Type: application/json\" \\");
            println!("      -d '{{\"model\":\"gpt-4\",\"messages\":[{{\"role\":\"user\",\"content\":\"hello\"}}],\"stream\":true}}'");
            println!();
            println!("  {}", "Benchmark:".cyan());
            println!("    oha -m POST --no-tui -H \"Content-Type: application/json\" \\");
            println!("      -d '{{\"prompt\":\"bench\"}}' -c 200 -n 2000 \\");
            println!("      http://localhost:{}{}", sink_port, sink_path);
        } else {
            println!(
                "    curl -X POST http://localhost:{}{} \\",
                sink_port, sink_path
            );
            println!("      -H \"Content-Type: application/json\" \\");
            println!("      -d '{{\"data\":\"test\"}}'");
        }
    }

    // Server-mode hints
    if !manifest.is_workflow() && !manifest.endpoints.is_empty() {
        let first_ep = &manifest.endpoints[0];
        println!();
        println!("  {}", "Test:".cyan());
        println!(
            "    curl -X {} http://localhost:{}{} \\",
            first_ep.method, manifest.port, first_ep.path
        );
        println!("      -H \"Content-Type: application/json\" \\");
        println!("      -d '{{\"data\":\"test\"}}'");
        println!();
        println!("  {}", "Benchmark:".cyan());
        println!(
            "    oha -n 10000 -c 50 http://localhost:{}{}",
            manifest.port, first_ep.path
        );
    }

    // Step 8 (optional): Generate .vlb artifact
    if config.target_vlb {
        println!("  {} Generating VLB artifact...", "[8/8]".dimmed());
        let vlb_path = generate_vlb_from_manifest(&manifest, &binary)?;
        println!("\n{} VLB artifact: {}", "OK".green().bold(), vlb_path);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Step 1: Obtain manifest YAML
// ---------------------------------------------------------------------------

fn obtain_manifest_yaml(from: &str, input: &str) -> Result<String, String> {
    match from {
        "yaml" => {
            println!("  {} Reading YAML manifest...", "[1/7]".dimmed());
            std::fs::read_to_string(input)
                .map_err(|e| format!("Failed to read YAML file '{}': {}", input, e))
        }
        lang => {
            println!(
                "  {} Running {} source with VIL_COMPILE_MODE=manifest...",
                "[1/7]".dimmed(),
                lang
            );
            run_source_for_manifest(lang, input)
        }
    }
}

/// Run a source file (python/typescript/go/java) with VIL_COMPILE_MODE=manifest
/// and capture YAML from stdout.
fn run_source_for_manifest(lang: &str, input: &str) -> Result<String, String> {
    let (cmd, args) = match lang {
        "python" => ("python3", vec![input.to_string()]),
        "typescript" => {
            // Try tsx first, fall back to ts-node, then npx tsx
            if which_exists("tsx") {
                ("tsx", vec![input.to_string()])
            } else if which_exists("ts-node") {
                ("ts-node", vec![input.to_string()])
            } else {
                ("npx", vec!["tsx".to_string(), input.to_string()])
            }
        }
        "go" => ("go", vec!["run".to_string(), input.to_string()]),
        "java" => ("java", vec![input.to_string()]),
        "csharp" => ("dotnet", vec!["script".to_string(), input.to_string()]),
        "kotlin" => ("kotlin", vec![input.to_string()]),
        "swift" => ("swift", vec![input.to_string()]),
        "zig" => ("zig", vec!["run".to_string(), input.to_string()]),
        other => return Err(format!("No runner configured for language '{}'", other)),
    };

    let output = Command::new(cmd)
        .args(&args)
        .env("VIL_COMPILE_MODE", "manifest")
        .output()
        .map_err(|e| format!("Failed to run '{}': {} (is '{}' installed?)", cmd, e, cmd))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Source command exited with {}:\n{}",
            output.status, stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        return Err(format!(
            "Source command produced no YAML output. Make sure your {} SDK \
             checks VIL_COMPILE_MODE=manifest and prints YAML to stdout.",
            lang
        ));
    }

    Ok(stdout)
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Step 4–5: Write build directory
// ---------------------------------------------------------------------------

fn find_workspace_root() -> Result<String, String> {
    // Walk up from CWD looking for workspace Cargo.toml with [workspace]
    let mut dir = std::env::current_dir()
        .map_err(|e| format!("Cannot determine current directory: {}", e))?;

    loop {
        let cargo = dir.join("Cargo.toml");
        if cargo.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo) {
                if content.contains("[workspace]") {
                    return Ok(dir.to_string_lossy().to_string());
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }

    // Fallback: assume current dir
    std::env::current_dir()
        .map(|d| d.to_string_lossy().to_string())
        .map_err(|e| format!("Cannot determine workspace root: {}", e))
}

fn write_build_dir(name: &str, rust_src: &str, cargo_toml: &str) -> Result<PathBuf, String> {
    let base = PathBuf::from(format!("/tmp/vil-compile-{}", name));

    // Clean previous build
    if base.exists() {
        // Remove src/ and Cargo.lock (force fresh dependency resolution)
        // Keep target/ for incremental builds
        let src_dir = base.join("src");
        if src_dir.exists() {
            std::fs::remove_dir_all(&src_dir)
                .map_err(|e| format!("Failed to clean src dir: {}", e))?;
        }
        let lock_file = base.join("Cargo.lock");
        if lock_file.exists() {
            let _ = std::fs::remove_file(&lock_file);
        }
    }

    let src_dir = base.join("src");
    std::fs::create_dir_all(&src_dir).map_err(|e| format!("Failed to create build dir: {}", e))?;

    // Write Cargo.toml
    std::fs::write(base.join("Cargo.toml"), cargo_toml)
        .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;

    // Write src/main.rs
    std::fs::write(src_dir.join("main.rs"), rust_src)
        .map_err(|e| format!("Failed to write main.rs: {}", e))?;

    Ok(base)
}

// ---------------------------------------------------------------------------
// Step 6: cargo build
// ---------------------------------------------------------------------------

fn cargo_build(build_dir: &Path, release: bool) -> Result<(), String> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if release {
        cmd.arg("--release");
    }
    cmd.current_dir(build_dir);
    // Suppress all warnings for clean user output
    cmd.env("RUSTFLAGS", "-Awarnings");

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;
    if !status.success() {
        return Err(format!("cargo build failed (exit code: {})", status));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Step 7: Copy binary
// ---------------------------------------------------------------------------

fn copy_binary(
    build_dir: &Path,
    crate_name: &str,
    output_name: &str,
    release: bool,
) -> Result<PathBuf, String> {
    let profile = if release { "release" } else { "debug" };
    // Cargo binary name: hyphens in crate name become underscores
    let bin_name = crate_name.replace('-', "_");
    let src_binary = build_dir.join("target").join(profile).join(&bin_name);

    if !src_binary.exists() {
        // Try with original name (some cargo versions keep hyphens)
        let alt = build_dir.join("target").join(profile).join(crate_name);
        if !alt.exists() {
            return Err(format!(
                "Binary not found at {} or {}",
                src_binary.display(),
                alt.display()
            ));
        }
        let dest = PathBuf::from(output_name);
        std::fs::copy(&alt, &dest).map_err(|e| format!("Failed to copy binary: {}", e))?;
        return Ok(dest);
    }

    let dest = PathBuf::from(output_name);
    std::fs::copy(&src_binary, &dest).map_err(|e| format!("Failed to copy binary: {}", e))?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
    }

    Ok(dest)
}

// ---------------------------------------------------------------------------
// Step 8: Generate .vlb artifact from manifest (3.5)
// ---------------------------------------------------------------------------

fn generate_vlb_from_manifest(
    manifest: &WorkflowManifest,
    binary_path: &Path,
) -> Result<String, String> {
    // Build service manifest JSON for VLB
    let endpoints_json: Vec<serde_json::Value> = manifest
        .endpoints
        .iter()
        .map(|ep| {
            serde_json::json!({
                "method": ep.method,
                "path": ep.path,
                "handler": ep.handler,
            })
        })
        .collect();

    let mesh_requires: Vec<serde_json::Value> = manifest
        .mesh
        .as_ref()
        .map(|m| {
            m.routes
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "from": r.from,
                        "to": r.to,
                        "lane": r.lane,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let state_type = manifest
        .state
        .as_ref()
        .map(|s| s.storage_type.as_str())
        .unwrap_or("None");

    let svc_manifest = serde_json::json!({
        "name": manifest.name,
        "version": manifest.vil_version,
        "description": format!("VIL service: {}", manifest.name),
        "port": manifest.port,
        "endpoints": endpoints_json,
        "ports": [
            { "name": "trigger_in", "lane": "Trigger", "transfer_mode": "LoanWrite", "direction": "In" },
            { "name": "data_out", "lane": "Data", "transfer_mode": "LoanWrite", "direction": "Out" },
            { "name": "ctrl_out", "lane": "Control", "transfer_mode": "Copy", "direction": "Out" },
        ],
        "mesh_requires": mesh_requires,
        "state_type": state_type,
        "min_shm_bytes": 4194304_u64,
        "exec_class_default": "AsyncTask",
    });

    let manifest_bytes = serde_json::to_vec(&svc_manifest).unwrap_or_default();

    // Read binary fingerprint
    let native_code = if binary_path.exists() {
        let data = std::fs::read(binary_path)
            .map_err(|e| format!("Failed to read binary for VLB: {}", e))?;
        let fingerprint = data.len() as u64;
        fingerprint.to_le_bytes().to_vec()
    } else {
        vec![0u8; 8]
    };

    // Build VLB binary (same format as vlb_builder)
    let schemas_bytes = b"[]".to_vec();
    let resources_bytes: Vec<u8> = vec![];

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

    let checksum: u32 = manifest_bytes
        .iter()
        .chain(schemas_bytes.iter())
        .chain(native_code.iter())
        .chain(resources_bytes.iter())
        .fold(0u32, |acc, &b| acc.wrapping_add(b as u32));

    let mut buf = Vec::new();
    buf.extend_from_slice(magic);
    buf.extend_from_slice(&version.to_le_bytes());
    buf.extend_from_slice(&arch.to_le_bytes());
    buf.extend_from_slice(&section_count.to_le_bytes());
    buf.extend_from_slice(&flags.to_le_bytes());
    buf.extend_from_slice(&checksum.to_le_bytes());

    for (id, offset, size) in [
        (1u16, s1_off, s1_len),
        (2u16, s2_off, s2_len),
        (4u16, s3_off, s3_len),
        (5u16, s4_off, s4_len),
    ] {
        buf.extend_from_slice(&id.to_le_bytes());
        buf.extend_from_slice(&offset.to_le_bytes());
        buf.extend_from_slice(&size.to_le_bytes());
    }

    buf.extend_from_slice(&manifest_bytes);
    buf.extend_from_slice(&schemas_bytes);
    buf.extend_from_slice(&native_code[..s3_len as usize]);
    buf.extend_from_slice(&resources_bytes);

    // Write .vlb file next to the binary
    let vlb_path = format!("{}.vlb", binary_path.display());
    std::fs::write(&vlb_path, &buf).map_err(|e| format!("Failed to write VLB artifact: {}", e))?;

    Ok(vlb_path)
}
