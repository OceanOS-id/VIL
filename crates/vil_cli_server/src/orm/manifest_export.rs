//! vil export-manifest — Parse Rust source → generate YAML manifest.
//!
//! Reads VilApp/ServiceProcess patterns from .rs files, emits WorkflowManifest YAML.
//! Zero runtime dependency — pure text parsing.

use std::path::Path;

/// Parsed VilApp from Rust source.
#[derive(Debug)]
pub struct ParsedApp {
    pub name: String,
    pub port: u16,
    pub services: Vec<ParsedService>,
}

/// Parsed ServiceProcess.
#[derive(Debug)]
pub struct ParsedService {
    pub name: String,
    pub endpoints: Vec<ParsedEndpoint>,
}

/// Parsed endpoint.
#[derive(Debug)]
pub struct ParsedEndpoint {
    pub method: String,
    pub path: String,
    pub handler: String,
}

/// Parse a Rust source file and extract VilApp structure.
pub fn parse_rust_source(path: &Path) -> Result<ParsedApp, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let name = extract_app_name(&source).unwrap_or_else(|| "app".to_string());
    let port = extract_port(&source).unwrap_or(8080);
    let services = extract_services(&source);

    Ok(ParsedApp { name, port, services })
}

/// Generate YAML manifest from parsed app.
pub fn to_manifest_yaml(app: &ParsedApp) -> String {
    let mut lines = Vec::new();
    lines.push(format!("vil_version: \"6.0.0\""));
    lines.push(format!("name: {}", app.name));
    lines.push(format!("port: {}", app.port));
    lines.push("mode: server".to_string());

    if !app.services.is_empty() {
        lines.push(String::new());
        lines.push("services:".to_string());
        for svc in &app.services {
            lines.push(format!("  - name: {}", svc.name));
            lines.push(format!("    prefix: /api/{}", svc.name));
            if !svc.endpoints.is_empty() {
                lines.push("    endpoints:".to_string());
                for ep in &svc.endpoints {
                    lines.push(format!("      - method: {}", ep.method));
                    lines.push(format!("        path: {}", ep.path));
                    lines.push(format!("        handler: {}", ep.handler));
                }
            }
        }
    }

    lines.join("\n") + "\n"
}

// ── Source Parsing Helpers ──

/// Extract app name from `VilApp::new("name")`
fn extract_app_name(source: &str) -> Option<String> {
    // Pattern: VilApp::new("name")
    for line in source.lines() {
        if let Some(pos) = line.find("VilApp::new(") {
            let after = &line[pos + 12..];
            if let Some(name) = extract_quoted(after) {
                return Some(name);
            }
        }
    }
    None
}

/// Extract port from `.port(NNNN)`
fn extract_port(source: &str) -> Option<u16> {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(".port(") {
            let inner = &trimmed[6..];
            if let Some(end) = inner.find(')') {
                return inner[..end].trim().parse().ok();
            }
        }
    }
    None
}

/// Extract all ServiceProcess definitions with their endpoints.
fn extract_services(source: &str) -> Vec<ParsedService> {
    let mut services = Vec::new();
    let mut current_service: Option<(String, Vec<ParsedEndpoint>)> = None;

    for line in source.lines() {
        let trimmed = line.trim();

        // Detect ServiceProcess::new("name")
        if let Some(pos) = trimmed.find("ServiceProcess::new(") {
            // Save previous service
            if let Some((name, endpoints)) = current_service.take() {
                services.push(ParsedService { name, endpoints });
            }
            let after = &trimmed[pos + 20..];
            if let Some(name) = extract_quoted(after) {
                current_service = Some((name, Vec::new()));
            }
        }

        // Detect .endpoint(Method::GET, "/path", get(handler))
        if trimmed.starts_with(".endpoint(") {
            if let Some((_, ref mut endpoints)) = current_service {
                if let Some(ep) = parse_endpoint_line(trimmed) {
                    endpoints.push(ep);
                }
            }
        }

        // Detect .state() or .service() as end of service definition
        if (trimmed.starts_with(".state(") || trimmed.starts_with(".service("))
            && current_service.is_some()
            && trimmed.starts_with(".state(")
        {
            // .state() is part of the service chain, continue
        }
    }

    // Save last service
    if let Some((name, endpoints)) = current_service {
        services.push(ParsedService { name, endpoints });
    }

    services
}

/// Parse `.endpoint(Method::GET, "/path", get(handler::func))` line.
fn parse_endpoint_line(line: &str) -> Option<ParsedEndpoint> {
    // .endpoint(Method::GET, "/path", get(module::handler))
    let inner = line.strip_prefix(".endpoint(")?;

    // Extract method
    let method = if inner.contains("Method::GET") {
        "GET"
    } else if inner.contains("Method::POST") {
        "POST"
    } else if inner.contains("Method::PUT") {
        "PUT"
    } else if inner.contains("Method::PATCH") {
        "PATCH"
    } else if inner.contains("Method::DELETE") {
        "DELETE"
    } else {
        return None;
    };

    // Extract path (second argument, quoted)
    let parts: Vec<&str> = inner.splitn(3, ',').collect();
    if parts.len() < 3 {
        return None;
    }
    let path = extract_quoted(parts[1]).unwrap_or_default();

    // Extract handler: get(module::func) or post(module::func)
    let handler_part = parts[2].trim();
    let handler = extract_handler_name(handler_part);

    Some(ParsedEndpoint {
        method: method.to_string(),
        path,
        handler,
    })
}

/// Extract handler name from `get(module::func)` or `post(module::func)`
fn extract_handler_name(s: &str) -> String {
    // Pattern: get(svc::handler) or post(svc::handler)
    for prefix in &["get(", "post(", "put(", "patch(", "delete("] {
        if let Some(pos) = s.find(prefix) {
            let after = &s[pos + prefix.len()..];
            if let Some(end) = after.find(')') {
                return after[..end].trim().to_string();
            }
        }
    }
    s.trim_end_matches(')').trim().to_string()
}

/// Extract first quoted string from text.
fn extract_quoted(s: &str) -> Option<String> {
    let start = s.find('"')? + 1;
    let end = s[start..].find('"')? + start;
    Some(s[start..end].to_string())
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let source = r#"
let svc = ServiceProcess::new("tasks")
    .endpoint(Method::GET, "/list", get(tasks_svc::list))
    .endpoint(Method::GET, "/:id", get(tasks_svc::get_by_id))
    .endpoint(Method::POST, "/create", post(tasks_svc::create))
    .endpoint(Method::PUT, "/:id", put(tasks_svc::update))
    .endpoint(Method::DELETE, "/:id", delete(tasks_svc::delete))
    .state(state.clone());

VilApp::new("my-app")
    .port(8080)
    .service(svc)
    .run().await;
        "#;
        let name = extract_app_name(source).unwrap();
        assert_eq!(name, "my-app");
        assert_eq!(extract_port(source), Some(8080));

        let services = extract_services(source);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "tasks");
        assert_eq!(services[0].endpoints.len(), 5);
        assert_eq!(services[0].endpoints[0].method, "GET");
        assert_eq!(services[0].endpoints[0].path, "/list");
        assert_eq!(services[0].endpoints[0].handler, "tasks_svc::list");
    }

    #[test]
    fn test_parse_multi_service() {
        let source = r#"
let auth = ServiceProcess::new("auth")
    .endpoint(Method::POST, "/login", post(auth::login))
    .endpoint(Method::POST, "/register", post(auth::register))
    .state(state.clone());

let blog = ServiceProcess::new("blog")
    .endpoint(Method::GET, "/posts", get(blog::list))
    .endpoint(Method::POST, "/posts", post(blog::create))
    .state(state.clone());

VilApp::new("my-server")
    .port(3000)
    .service(auth)
    .service(blog)
    .run().await;
        "#;
        let services = extract_services(source);
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].name, "auth");
        assert_eq!(services[0].endpoints.len(), 2);
        assert_eq!(services[1].name, "blog");
        assert_eq!(services[1].endpoints.len(), 2);
    }

    #[test]
    fn test_yaml_output() {
        let app = ParsedApp {
            name: "test-app".to_string(),
            port: 8080,
            services: vec![ParsedService {
                name: "tasks".to_string(),
                endpoints: vec![
                    ParsedEndpoint { method: "GET".into(), path: "/list".into(), handler: "list".into() },
                    ParsedEndpoint { method: "POST".into(), path: "/create".into(), handler: "create".into() },
                ],
            }],
        };
        let yaml = to_manifest_yaml(&app);
        assert!(yaml.contains("vil_version: \"6.0.0\""));
        assert!(yaml.contains("name: test-app"));
        assert!(yaml.contains("port: 8080"));
        assert!(yaml.contains("mode: server"));
        assert!(yaml.contains("- name: tasks"));
        assert!(yaml.contains("method: GET"));
        assert!(yaml.contains("path: /list"));
    }

    #[test]
    fn test_parse_real_example() {
        let path = std::path::Path::new(
            "/home/abraham/Prdmid/vil-project/vil/examples/004-basic-rest-crud/src/main.rs"
        );
        if path.exists() {
            let app = parse_rust_source(path).expect("parse 004");
            assert_eq!(app.name, "crud-vilorm");
            assert_eq!(app.port, 8080);
            assert!(app.services.len() >= 1);
            assert!(app.services[0].endpoints.len() >= 5);
            println!("004 parsed: {} services, {} endpoints",
                app.services.len(), app.services[0].endpoints.len());
            println!("{}", to_manifest_yaml(&app));
        }
    }
}
