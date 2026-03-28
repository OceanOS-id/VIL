//! VLB Inspector — inspect .vlb artifacts and service topology

pub fn inspect_vlb(
    path: &str,
    show_contract: bool,
    show_routes: bool,
    show_processes: bool,
    show_schemas: bool,
) -> Result<(), String> {
    // Read VLB file
    let data = std::fs::read(path).map_err(|e| format!("Cannot read '{}': {}", path, e))?;

    // Check magic
    if data.len() < 16 || &data[0..4] != b"VLNG" {
        return Err(format!("'{}' is not a valid VLB file", path));
    }

    // Parse header manually (avoid dependency on vflow_server)
    let version = u16::from_le_bytes([data[4], data[5]]);
    let arch = u16::from_le_bytes([data[6], data[7]]);
    let section_count = u16::from_le_bytes([data[8], data[9]]);
    let checksum = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

    let arch_str = match arch {
        1 => "x86_64",
        2 => "aarch64",
        3 => "wasm32",
        _ => "unknown",
    };

    // Parse section table
    let mut manifest_data: Option<&[u8]> = None;
    let mut schemas_data: Option<&[u8]> = None;
    let mut code_size: usize = 0;
    let mut resource_size: usize = 0;

    for i in 0..section_count as usize {
        let base = 16 + i * 8;
        if base + 8 > data.len() {
            break;
        }
        let id = u16::from_le_bytes([data[base], data[base + 1]]);
        let offset = u32::from_le_bytes([
            data[base + 2],
            data[base + 3],
            data[base + 4],
            data[base + 5],
        ]) as usize;
        let size = u16::from_le_bytes([data[base + 6], data[base + 7]]) as usize;

        if offset + size <= data.len() {
            match id {
                1 => manifest_data = Some(&data[offset..offset + size]), // Manifest
                2 => schemas_data = Some(&data[offset..offset + size]),  // Schemas
                4 => code_size = size,                                   // NativeCode
                5 => resource_size = size,                               // Resources
                _ => {}
            }
        }
    }

    // Parse manifest JSON
    let manifest: serde_json::Value = manifest_data
        .and_then(|d| serde_json::from_slice(d).ok())
        .unwrap_or(serde_json::json!({}));

    let name = manifest["name"].as_str().unwrap_or("unknown");
    let ver = manifest["version"].as_str().unwrap_or("0.0.0");
    let desc = manifest["description"].as_str().unwrap_or("");

    // Always show header
    println!();
    println!("  ═══════════════════════════════════════════════════");
    println!("  VIL Binary: {} v{}", name, ver);
    println!("  ═══════════════════════════════════════════════════");
    println!("  Architecture:  {}", arch_str);
    println!("  VLB Version:   {}", version);
    println!("  Sections:      {}", section_count);
    println!("  File Size:     {} bytes", data.len());
    println!("  Checksum:      0x{:08X}", checksum);
    if !desc.is_empty() {
        println!("  Description:   {}", desc);
    }

    // Endpoints
    if let Some(endpoints) = manifest["endpoints"].as_array() {
        println!();
        println!("  Endpoints ({}):", endpoints.len());
        for ep in endpoints {
            let method = ep["method"].as_str().unwrap_or("?");
            let path = ep["path"].as_str().unwrap_or("?");
            let handler = ep["handler_name"].as_str().unwrap_or("?");
            let exec = ep["exec_class"].as_str().unwrap_or("AsyncTask");
            println!("    {:>7} {:<30} → {} [{}]", method, path, handler, exec);
        }
    }

    // Ports
    if let Some(ports) = manifest["ports"].as_array() {
        if show_processes || show_routes {
            println!();
            println!("  Ports ({}):", ports.len());
            for port in ports {
                let pname = port["name"].as_str().unwrap_or("?");
                let lane = port["lane"].as_str().unwrap_or("?");
                let mode = port["transfer_mode"].as_str().unwrap_or("?");
                let dir = port["direction"].as_str().unwrap_or("?");
                println!("    {:<20} {} {:>10} ({})", pname, dir, lane, mode);
            }
        }
    }

    // Mesh requires
    if let Some(mesh) = manifest["mesh_requires"].as_array() {
        if !mesh.is_empty() && (show_routes || show_contract) {
            println!();
            println!("  Mesh Requires:");
            for req in mesh {
                let target = req["target_service"].as_str().unwrap_or("?");
                let lane = req["lane"].as_str().unwrap_or("?");
                println!("    → {} ({} Lane)", target, lane);
            }
        }
    }

    // State type
    let state_type = manifest["state_type"]
        .as_str()
        .or_else(|| {
            manifest["state_type"]
                .as_object()
                .and_then(|_| Some("custom"))
        })
        .unwrap_or("None");
    let min_shm = manifest["min_shm_bytes"].as_u64().unwrap_or(0);

    if show_processes {
        println!();
        println!("  State:         {}", state_type);
        println!(
            "  Min SHM:       {} bytes ({} MB)",
            min_shm,
            min_shm / (1024 * 1024)
        );
        println!("  Code Section:  {} bytes", code_size);
        println!("  Resources:     {} bytes", resource_size);
    }

    // Schemas
    if show_schemas {
        if let Some(schema_bytes) = schemas_data {
            if let Ok(schemas) = serde_json::from_slice::<Vec<serde_json::Value>>(schema_bytes) {
                if !schemas.is_empty() {
                    println!();
                    println!("  Message Schemas ({}):", schemas.len());
                    for schema in &schemas {
                        let sname = schema["name"].as_str().unwrap_or("?");
                        if let Some(fields) = schema["fields"].as_array() {
                            let fields_str: Vec<String> = fields
                                .iter()
                                .map(|f| {
                                    let fname = f["name"].as_str().unwrap_or("?");
                                    let ftype = f["field_type"].as_str().unwrap_or("?");
                                    format!("{}: {}", fname, ftype)
                                })
                                .collect();
                            println!("    {} {{ {} }}", sname, fields_str.join(", "));
                        }
                    }
                }
            }
        }
    }

    // Full contract JSON
    if show_contract {
        println!();
        println!("  Contract JSON:");
        if let Ok(pretty) = serde_json::to_string_pretty(&manifest) {
            for line in pretty.lines() {
                println!("    {}", line);
            }
        }
    }

    println!();
    Ok(())
}

/// Inspect current project topology (scan Cargo.toml + src/)
pub fn inspect_project(
    show_contract: bool,
    show_routes: bool,
    show_processes: bool,
    show_schemas: bool,
) -> Result<(), String> {
    let cargo_toml = std::fs::read_to_string("Cargo.toml")
        .map_err(|e| format!("Cannot read Cargo.toml: {}. Run from project root.", e))?;

    let mut name = "unknown".to_string();
    let mut version = "0.0.0".to_string();
    for line in cargo_toml.lines() {
        let t = line.trim();
        if t.starts_with("name") {
            if let Some(v) = t.split('=').nth(1) {
                name = v.trim().trim_matches('"').to_string();
            }
        }
        if t.starts_with("version") {
            if let Some(v) = t.split('=').nth(1) {
                version = v.trim().trim_matches('"').to_string();
            }
        }
    }

    // Scan source files for VX patterns
    let src = scan_src("src/");

    let has_vil_app = src.contains("VilApp") || src.contains("vil_app!");
    let has_service_process = src.contains("ServiceProcess");
    let has_vil_endpoint = src.contains("vil_endpoint");
    let has_vil_model = src.contains("VilModel");
    let has_mesh = src.contains("VxMeshConfig") || src.contains("vil_service");
    let has_tri_lane =
        src.contains("VxLane") || src.contains("Lane::Data") || src.contains("Lane::Trigger");

    println!();
    println!("  ═══════════════════════════════════════════════════");
    println!("  VIL Project: {} v{}", name, version);
    println!("  ═══════════════════════════════════════════════════");
    println!();
    println!("  VX Architecture:");
    println!(
        "    VilApp:         {}",
        if has_vil_app { "yes" } else { "no" }
    );
    println!(
        "    ServiceProcess:   {}",
        if has_service_process { "yes" } else { "no" }
    );
    println!(
        "    #[vil_endpoint]:{}",
        if has_vil_endpoint { " yes" } else { " no" }
    );
    println!(
        "    VilModel:       {}",
        if has_vil_model { "yes" } else { "no" }
    );
    println!(
        "    Tri-Lane Mesh:    {}",
        if has_mesh || has_tri_lane {
            "yes"
        } else {
            "no"
        }
    );

    // Count endpoints (rough -- count occurrences of vil_endpoint)
    let endpoint_count = src.matches("vil_endpoint").count();
    let model_count = src.matches("VilModel").count();

    println!();
    println!("  Statistics:");
    println!("    Endpoints:    ~{}", endpoint_count);
    println!("    VilModels:  ~{}", model_count);

    // VLB readiness
    println!();
    println!("  VLB Readiness:");
    println!(
        "    [{}] #[vil_endpoint] on handlers",
        if has_vil_endpoint { "ok" } else { "--" }
    );
    println!(
        "    [{}] VilModel on types",
        if has_vil_model { "ok" } else { "--" }
    );
    println!(
        "    [{}] VilApp / vil_app!",
        if has_vil_app { "ok" } else { "--" }
    );

    let vlb_ready = has_vil_endpoint && has_vil_model && has_vil_app;
    if vlb_ready {
        println!();
        println!("  -> Ready for: vil build --target vlb");
    } else {
        println!();
        println!("  -> Not yet VLB-ready. Fix items marked '--' above.");
    }

    let _ = (show_contract, show_routes, show_processes, show_schemas);

    println!();
    Ok(())
}

fn scan_src(dir: &str) -> String {
    let mut content = String::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(c) = std::fs::read_to_string(&path) {
                    content.push_str(&c);
                }
            } else if path.is_dir() {
                content.push_str(&scan_src(path.to_str().unwrap_or("")));
            }
        }
    }
    content
}
