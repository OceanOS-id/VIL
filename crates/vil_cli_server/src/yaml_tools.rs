//! YAML export/scaffold tools for VIL services
//!
//! Export: running topology → YAML representation
//! Scaffold: YAML → Rust code skeleton

pub struct ExportConfig {
    pub output: Option<String>,
}

pub struct ScaffoldConfig {
    pub input: String,
    pub output: Option<String>,
}

/// Export current project as YAML topology
pub fn export_yaml(config: ExportConfig) -> Result<(), String> {
    // Read Cargo.toml for project metadata
    let cargo_toml = std::fs::read_to_string("Cargo.toml")
        .map_err(|e| format!("Cannot read Cargo.toml: {}", e))?;

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

    // Scan source for VX patterns
    let src = scan_src("src/");

    // Extract service names from ServiceProcess::new("...") or vil_service(name = "...")
    let services = extract_services(&src);

    // Extract mesh routes from VxMeshConfig
    let mesh_routes = extract_mesh_routes(&src);

    // Extract endpoints from vil_endpoint
    let _endpoints = extract_endpoints(&src);

    // Build YAML
    let mut yaml = String::new();
    yaml.push_str(&format!("# VIL Service Topology\n"));
    yaml.push_str(&format!("# Exported from: {} v{}\n", name, version));
    yaml.push_str(&format!("# Date: {}\n\n", chrono_now()));

    yaml.push_str(&format!("name: {}\n", name));
    yaml.push_str(&format!("version: \"{}\"\n", version));
    yaml.push_str("port: 8080\n\n");

    // Services
    if !services.is_empty() {
        yaml.push_str("services:\n");
        for svc in &services {
            yaml.push_str(&format!("  - name: {}\n", svc.name));
            yaml.push_str(&format!("    visibility: {}\n", svc.visibility));
            if !svc.prefix.is_empty() {
                yaml.push_str(&format!("    prefix: \"{}\"\n", svc.prefix));
            }
            if !svc.endpoints.is_empty() {
                yaml.push_str("    endpoints:\n");
                for ep in &svc.endpoints {
                    yaml.push_str(&format!("      - method: {}\n", ep.method));
                    yaml.push_str(&format!("        path: \"{}\"\n", ep.path));
                    yaml.push_str(&format!("        handler: {}\n", ep.handler));
                }
            }
        }
        yaml.push('\n');
    }

    // Mesh
    if !mesh_routes.is_empty() {
        yaml.push_str("mesh:\n");
        yaml.push_str("  routes:\n");
        for route in &mesh_routes {
            yaml.push_str(&format!("    - from: {}\n", route.0));
            yaml.push_str(&format!("      to: {}\n", route.1));
            yaml.push_str(&format!("      lane: {}\n", route.2));
        }
        yaml.push('\n');
    }

    // Output
    match config.output {
        Some(path) => {
            std::fs::write(&path, &yaml).map_err(|e| format!("Cannot write {}: {}", path, e))?;
            println!("Exported topology to: {}", path);
        }
        None => {
            print!("{}", yaml);
        }
    }

    Ok(())
}

/// Scaffold Rust code from YAML topology
pub fn scaffold_yaml(config: ScaffoldConfig) -> Result<(), String> {
    let yaml_content = std::fs::read_to_string(&config.input)
        .map_err(|e| format!("Cannot read {}: {}", config.input, e))?;

    // Parse YAML (simple line-based parser — no serde_yaml dependency in CLI)
    let mut name = "my-service".to_string();
    let mut port: u16 = 8080;
    let mut services: Vec<ScaffoldService> = Vec::new();
    let mut current_service: Option<ScaffoldService> = None;
    let mut in_services = false;
    let mut in_endpoints = false;

    for line in yaml_content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("name:") {
            name = trimmed.split(':').nth(1).unwrap_or("").trim().to_string();
        }
        if trimmed.starts_with("port:") {
            port = trimmed
                .split(':')
                .nth(1)
                .unwrap_or("8080")
                .trim()
                .parse()
                .unwrap_or(8080);
        }
        if trimmed == "services:" {
            in_services = true;
            in_endpoints = false;
            continue;
        }
        if in_services && trimmed.starts_with("- name:") {
            if let Some(svc) = current_service.take() {
                services.push(svc);
            }
            let svc_name = trimmed
                .strip_prefix("- name:")
                .unwrap_or("")
                .trim()
                .to_string();
            current_service = Some(ScaffoldService {
                name: svc_name,
                prefix: String::new(),
                endpoints: Vec::new(),
            });
            in_endpoints = false;
        }
        if in_services && trimmed.starts_with("prefix:") {
            if let Some(ref mut svc) = current_service {
                svc.prefix = trimmed
                    .split(':')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_string();
            }
        }
        if trimmed == "endpoints:" {
            in_endpoints = true;
            continue;
        }
        if in_endpoints && trimmed.starts_with("- method:") {
            let method = trimmed
                .strip_prefix("- method:")
                .unwrap_or("GET")
                .trim()
                .to_string();
            if let Some(ref mut svc) = current_service {
                svc.endpoints.push(ScaffoldEndpoint {
                    method,
                    path: String::new(),
                    handler: String::new(),
                });
            }
        }
        if in_endpoints && trimmed.starts_with("path:") {
            if let Some(ref mut svc) = current_service {
                if let Some(ep) = svc.endpoints.last_mut() {
                    ep.path = trimmed
                        .split(':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .trim_matches('"')
                        .to_string();
                }
            }
        }
        if in_endpoints && trimmed.starts_with("handler:") {
            if let Some(ref mut svc) = current_service {
                if let Some(ep) = svc.endpoints.last_mut() {
                    ep.handler = trimmed.split(':').nth(1).unwrap_or("").trim().to_string();
                }
            }
        }
        if trimmed.starts_with("mesh:") || trimmed.starts_with("---") {
            in_services = false;
            in_endpoints = false;
        }
    }
    if let Some(svc) = current_service {
        services.push(svc);
    }

    // Generate Rust code
    let mut code = String::new();
    code.push_str("//! Auto-generated by: vil scaffold\n");
    code.push_str(&format!("//! Source: {}\n\n", config.input));
    code.push_str("use vil_server::prelude::*;\n\n");

    // Generate handler stubs
    for svc in &services {
        code.push_str(&format!("// ── Service: {} ──\n\n", svc.name));
        for ep in &svc.endpoints {
            let fn_name = if ep.handler.is_empty() {
                ep.path
                    .replace('/', "_")
                    .replace(':', "")
                    .trim_start_matches('_')
                    .to_string()
            } else {
                ep.handler.clone()
            };
            code.push_str("#[vil_endpoint]\n");
            code.push_str(&format!("async fn {}() -> &'static str {{\n", fn_name));
            code.push_str(&format!(
                "    \"TODO: implement {} {}\"\n",
                ep.method, ep.path
            ));
            code.push_str("}\n\n");
        }
    }

    // Generate main
    code.push_str("#[tokio::main]\n");
    code.push_str("async fn main() {\n");

    for svc in &services {
        let var_name = svc.name.replace('-', "_");
        code.push_str(&format!(
            "    let {} = ServiceProcess::new(\"{}\")\n",
            var_name, svc.name
        ));
        if !svc.prefix.is_empty() {
            code.push_str(&format!("        .prefix(\"{}\")\n", svc.prefix));
        }
        for ep in &svc.endpoints {
            let fn_name = if ep.handler.is_empty() {
                ep.path
                    .replace('/', "_")
                    .replace(':', "")
                    .trim_start_matches('_')
                    .to_string()
            } else {
                ep.handler.clone()
            };
            let method_fn = ep.method.to_lowercase();
            code.push_str(&format!(
                "        .endpoint(Method::{}, \"{}\", {}({}))\n",
                ep.method, ep.path, method_fn, fn_name
            ));
        }
        code.push_str("        ;\n\n");
    }

    code.push_str(&format!("    VilApp::new(\"{}\")\n", name));
    code.push_str(&format!("        .port({})\n", port));
    for svc in &services {
        let var_name = svc.name.replace('-', "_");
        code.push_str(&format!("        .service({})\n", var_name));
    }
    code.push_str("        .run()\n");
    code.push_str("        .await;\n");
    code.push_str("}\n");

    // Output
    match config.output {
        Some(path) => {
            std::fs::write(&path, &code).map_err(|e| format!("Cannot write {}: {}", path, e))?;
            println!("Scaffolded to: {}", path);
        }
        None => {
            print!("{}", code);
        }
    }

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────

struct ServiceInfo {
    name: String,
    visibility: String,
    prefix: String,
    endpoints: Vec<EndpointInfo>,
}

struct EndpointInfo {
    method: String,
    path: String,
    handler: String,
}

struct ScaffoldService {
    name: String,
    prefix: String,
    endpoints: Vec<ScaffoldEndpoint>,
}

struct ScaffoldEndpoint {
    method: String,
    path: String,
    handler: String,
}

fn extract_services(src: &str) -> Vec<ServiceInfo> {
    let mut services = Vec::new();
    // Look for ServiceProcess::new("name") patterns
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(start) = trimmed.find("ServiceProcess::new(\"") {
            let rest = &trimmed[start + 21..];
            if let Some(end) = rest.find('"') {
                let name = &rest[..end];
                services.push(ServiceInfo {
                    name: name.to_string(),
                    visibility: "public".to_string(),
                    prefix: String::new(),
                    endpoints: Vec::new(),
                });
            }
        }
    }
    services
}

fn extract_mesh_routes(src: &str) -> Vec<(String, String, String)> {
    let mut routes = Vec::new();
    // Look for .route("from", "to", VxLane::Data) or .route("from", "to", Lane::Data)
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.contains(".route(")
            && (trimmed.contains("Lane::") || trimmed.contains("VxLane::"))
        {
            // Simple extraction — parse quoted strings
            let parts: Vec<&str> = trimmed.split('"').collect();
            if parts.len() >= 5 {
                let from = parts[1].to_string();
                let to = parts[3].to_string();
                let lane = if trimmed.contains("Data") {
                    "data"
                } else if trimmed.contains("Trigger") {
                    "trigger"
                } else {
                    "control"
                };
                routes.push((from, to, lane.to_string()));
            }
        }
    }
    routes
}

fn extract_endpoints(_src: &str) -> Vec<(String, String)> {
    // Phase 1: return empty — full endpoint extraction requires AST parsing
    Vec::new()
}

fn scan_src(dir: &str) -> String {
    let mut content = String::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(c) = std::fs::read_to_string(&path) {
                    content.push_str(&c);
                    content.push('\n');
                }
            } else if path.is_dir() {
                content.push_str(&scan_src(path.to_str().unwrap_or("")));
            }
        }
    }
    content
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}s since epoch", dur.as_secs())
}
