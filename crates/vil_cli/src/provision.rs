//! vil provision — manage services on vil-server and vflow-server
//!
//! New commands (vil-server /api/admin/*):
//!   inspect  — scan workflow YAML, list handler requirements
//!   upload   — upload .so + .wasm + YAML to running server
//!   status   — show provisioned inventory (workflows, handlers)
//!
//! Legacy commands (vflow-server /internal/*):
//!   push, activate, drain, deactivate, list, contract, health

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use colored::*;

// ═══════════════════════════════════════════════════════════════════
// Inspect — scan workflow YAML for handler requirements
// ═══════════════════════════════════════════════════════════════════

/// A handler requirement extracted from a workflow YAML.
#[derive(Debug)]
pub struct HandlerReq {
    pub workflow_id: String,
    pub endpoint: String,     // e.g. "GET /api/banking/accounts"
    pub handler_type: String, // NativeCode, Function (WASM), Sidecar, Connector
    pub handler_ref: String,  // handler_ref, module_ref, or target name
}

/// Scan a path (file, dir, or project) for workflow YAML files and extract
/// all handler requirements.
pub fn scan_workflow_handlers(path: &str) -> Result<Vec<HandlerReq>, String> {
    let p = Path::new(path);
    let yaml_files = collect_yaml_files(p)?;
    if yaml_files.is_empty() {
        return Err(format!("No workflow YAML files found in '{}'", path));
    }

    let mut reqs = Vec::new();
    for yaml_path in &yaml_files {
        let content = std::fs::read_to_string(yaml_path)
            .map_err(|e| format!("read {}: {}", yaml_path.display(), e))?;

        // A single file may contain multiple YAML documents (--- separator)
        for doc in content.split("\n---") {
            let doc = doc.trim();
            if doc.is_empty() { continue; }
            if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(doc) {
                extract_handlers_from_yaml(&val, &mut reqs);
            }
        }
    }
    Ok(reqs)
}

fn collect_yaml_files(p: &Path) -> Result<Vec<PathBuf>, String> {
    if p.is_file() && p.extension().map_or(false, |e| e == "yaml" || e == "yml") {
        return Ok(vec![p.to_path_buf()]);
    }
    if p.is_dir() {
        // If project dir, look for workflows/ subdir first
        let workflows_dir = p.join("workflows");
        let vwfd_workflows = p.join("vwfd").join("workflows");
        let search_dir = if workflows_dir.is_dir() {
            workflows_dir
        } else if vwfd_workflows.is_dir() {
            vwfd_workflows
        } else {
            p.to_path_buf()
        };
        let mut files = Vec::new();
        collect_yaml_recursive(&search_dir, &mut files);
        files.sort();
        return Ok(files);
    }
    Err(format!("'{}' is not a file or directory", p.display()))
}

fn collect_yaml_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_yaml_recursive(&path, files);
            } else if path.extension().map_or(false, |e| e == "yaml" || e == "yml") {
                files.push(path);
            }
        }
    }
}

fn extract_handlers_from_yaml(val: &serde_yaml::Value, reqs: &mut Vec<HandlerReq>) {
    let workflow_id = val.get("metadata")
        .and_then(|m| m.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let activities = match val.get("spec").and_then(|s| s.get("activities")).and_then(|a| a.as_sequence()) {
        Some(a) => a,
        None => return,
    };

    // Extract endpoint from trigger
    let mut endpoint = String::new();
    for act in activities {
        if let Some(tc) = act.get("trigger_config").and_then(|t| t.get("webhook_config")) {
            let method = tc.get("method").and_then(|v| v.as_str()).unwrap_or("POST");
            let path = tc.get("path").and_then(|v| v.as_str()).unwrap_or("");
            endpoint = format!("{} {}", method, path);
        }
    }

    for act in activities {
        let activity_type = act.get("activity_type").and_then(|v| v.as_str()).unwrap_or("");
        match activity_type {
            "NativeCode" => {
                let handler_ref = act.get("code_config")
                    .and_then(|c| c.get("handler_ref"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !handler_ref.is_empty() {
                    reqs.push(HandlerReq {
                        workflow_id: workflow_id.clone(),
                        endpoint: endpoint.clone(),
                        handler_type: "NativeCode".into(),
                        handler_ref,
                    });
                }
            }
            "Function" => {
                let module_ref = act.get("wasm_config")
                    .and_then(|c| c.get("module_ref"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !module_ref.is_empty() {
                    reqs.push(HandlerReq {
                        workflow_id: workflow_id.clone(),
                        endpoint: endpoint.clone(),
                        handler_type: "WASM".into(),
                        handler_ref: module_ref,
                    });
                }
            }
            "Sidecar" => {
                let target = act.get("sidecar_config")
                    .and_then(|c| c.get("target"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !target.is_empty() {
                    reqs.push(HandlerReq {
                        workflow_id: workflow_id.clone(),
                        endpoint: endpoint.clone(),
                        handler_type: "Sidecar".into(),
                        handler_ref: target,
                    });
                }
            }
            "Connector" => {
                let connector_type = act.get("connector_config")
                    .and_then(|c| c.get("connector_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !connector_type.is_empty() {
                    reqs.push(HandlerReq {
                        workflow_id: workflow_id.clone(),
                        endpoint: endpoint.clone(),
                        handler_type: "Connector".into(),
                        handler_ref: connector_type,
                    });
                }
            }
            _ => {}
        }
    }
}

pub fn run_inspect(
    path: &str,
    check_dir: bool,
    format: &str,
    plugin_dir: &str,
    wasm_dir: &str,
) -> Result<(), String> {
    let reqs = scan_workflow_handlers(path)?;

    if format == "json" {
        let json_arr: Vec<serde_json::Value> = reqs.iter().map(|r| {
            let mut obj = serde_json::Map::new();
            obj.insert("workflow_id".into(), serde_json::Value::String(r.workflow_id.clone()));
            obj.insert("endpoint".into(), serde_json::Value::String(r.endpoint.clone()));
            obj.insert("handler_type".into(), serde_json::Value::String(r.handler_type.clone()));
            obj.insert("handler_ref".into(), serde_json::Value::String(r.handler_ref.clone()));
            if check_dir {
                obj.insert("ready".into(), serde_json::Value::Bool(
                    check_handler_ready(r, plugin_dir, wasm_dir)
                ));
            }
            serde_json::Value::Object(obj)
        }).collect();
        println!("{}", serde_json::to_string_pretty(&json_arr).unwrap_or_default());
        return Ok(());
    }

    // Table format
    // Group by workflow
    let mut by_workflow: Vec<(&str, Vec<&HandlerReq>)> = Vec::new();
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for r in &reqs {
        if let Some(&idx) = seen.get(r.workflow_id.as_str()) {
            by_workflow[idx].1.push(r);
        } else {
            seen.insert(&r.workflow_id, by_workflow.len());
            by_workflow.push((&r.workflow_id, vec![r]));
        }
    }

    for (wf_id, handlers) in &by_workflow {
        let ep = handlers.first().map(|h| h.endpoint.as_str()).unwrap_or("");
        println!("  {} {} {}", "Workflow:".cyan().bold(), wf_id.bold(), ep.dimmed());
        for h in handlers {
            let status = if check_dir {
                if check_handler_ready(h, plugin_dir, wasm_dir) {
                    format!("[{} ✓]", "ready".green())
                } else {
                    format!("[{}]", "missing".red())
                }
            } else {
                String::new()
            };
            println!("    → {}: {}  {}", h.handler_type.yellow(), h.handler_ref, status);
        }
    }

    // Summary
    let mut native_count = 0;
    let mut wasm_count = 0;
    let mut sidecar_count = 0;
    let mut connector_count = 0;
    let mut missing_count = 0;
    for r in &reqs {
        match r.handler_type.as_str() {
            "NativeCode" => native_count += 1,
            "WASM" => wasm_count += 1,
            "Sidecar" => sidecar_count += 1,
            "Connector" => connector_count += 1,
            _ => {}
        }
        if check_dir && !check_handler_ready(r, plugin_dir, wasm_dir) {
            missing_count += 1;
        }
    }

    println!();
    println!("  {} {} NativeCode, {} WASM, {} Sidecar, {} Connector",
        "Summary:".cyan().bold(), native_count, wasm_count, sidecar_count, connector_count);
    if check_dir {
        if missing_count == 0 {
            println!("  {} All handlers ready", "✓".green().bold());
        } else {
            println!("  {} {} handler(s) missing", "✗".red().bold(), missing_count);
        }
    }

    Ok(())
}

fn check_handler_ready(req: &HandlerReq, plugin_dir: &str, wasm_dir: &str) -> bool {
    match req.handler_type.as_str() {
        "NativeCode" => Path::new(plugin_dir).join(format!("{}.so", req.handler_ref)).exists(),
        "WASM" => Path::new(wasm_dir).join(format!("{}.wasm", req.handler_ref)).exists(),
        "Sidecar" => true, // sidecars are spawned from command, no file check
        "Connector" => true, // connectors are infra, no file check
        _ => true,
    }
}

// ═══════════════════════════════════════════════════════════════════
// Upload — send handlers + workflows to running vil-server
// ═══════════════════════════════════════════════════════════════════

pub fn run_upload(
    path: &str,
    host: &str,
    key: Option<&str>,
    plugin_dir: &str,
    wasm_dir: &str,
    handlers_only: bool,
    workflows_only: bool,
    activate: bool,
    timeout: u64,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;

    // Health check
    print!("  Checking server health... ");
    let health_url = format!("{}/api/admin/health", host);
    let resp = client.get(&health_url).send()
        .map_err(|e| format!("Server not reachable at {}: {}", host, e))?;
    if !resp.status().is_success() {
        return Err(format!("Server unhealthy: HTTP {}", resp.status()));
    }
    println!("{}", "OK".green().bold());

    let mut upload_ok = 0u32;
    let mut upload_fail = 0u32;

    // Upload .so plugins
    if !workflows_only {
        let plugin_path = Path::new(plugin_dir);
        if plugin_path.is_dir() {
            let mut so_files: Vec<_> = std::fs::read_dir(plugin_path)
                .map_err(|e| format!("read plugin dir: {}", e))?
                .flatten()
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "so"))
                .collect();
            so_files.sort_by_key(|e| e.file_name());

            if !so_files.is_empty() {
                println!("\n  {} Uploading {} NativeCode plugin(s)", "→".cyan(), so_files.len());
            }
            for entry in &so_files {
                let handler_name = entry.path().file_stem()
                    .unwrap_or_default().to_string_lossy().to_string();
                let url = format!("{}/api/admin/upload/plugin", host);
                let file_path = entry.path().to_string_lossy().to_string();
                match upload_binary_file(&url, &file_path, "X-Handler-Ref", &handler_name, key) {
                    Ok(body) => {
                        if body.contains("\"handler\"") {
                            println!("    {} {}", "OK".green(), handler_name);
                            upload_ok += 1;
                        } else {
                            println!("    {} {} — {}", "FAIL".red(), handler_name, &body[..body.len().min(80)]);
                            upload_fail += 1;
                        }
                    }
                    Err(e) => {
                        println!("    {} {} — {}", "FAIL".red(), handler_name, e);
                        upload_fail += 1;
                    }
                }
            }
        }

        // Upload .wasm modules
        let wasm_path = Path::new(wasm_dir);
        if wasm_path.is_dir() {
            let mut wasm_files: Vec<_> = std::fs::read_dir(wasm_path)
                .map_err(|e| format!("read wasm dir: {}", e))?
                .flatten()
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "wasm"))
                .collect();
            wasm_files.sort_by_key(|e| e.file_name());

            if !wasm_files.is_empty() {
                println!("\n  {} Uploading {} WASM module(s)", "→".cyan(), wasm_files.len());
            }
            for entry in &wasm_files {
                let module_name = entry.path().file_stem()
                    .unwrap_or_default().to_string_lossy().to_string();
                let url = format!("{}/api/admin/upload/wasm", host);
                let file_path = entry.path().to_string_lossy().to_string();
                match upload_binary_file(&url, &file_path, "X-Module-Ref", &module_name, key) {
                    Ok(body) => {
                        if body.contains("\"module\"") {
                            println!("    {} {}", "OK".green(), module_name);
                            upload_ok += 1;
                        } else {
                            println!("    {} {} — {}", "FAIL".red(), module_name, &body[..body.len().min(80)]);
                            upload_fail += 1;
                        }
                    }
                    Err(e) => {
                        println!("    {} {} — {}", "FAIL".red(), module_name, e);
                        upload_fail += 1;
                    }
                }
            }
        }
    }

    // Upload workflow YAMLs
    if !handlers_only {
        let p = Path::new(path);
        let yaml_files = collect_yaml_files(p)?;
        if !yaml_files.is_empty() {
            println!("\n  {} Uploading {} workflow(s)", "→".cyan(), yaml_files.len());
        }

        let mut uploaded_ids: Vec<String> = Vec::new();

        for yaml_path in &yaml_files {
            let wf_name = yaml_path.file_stem()
                .unwrap_or_default().to_string_lossy().to_string();
            let content = std::fs::read(yaml_path)
                .map_err(|e| format!("read {}: {}", yaml_path.display(), e))?;
            let url = format!("{}/api/admin/upload", host);
            let mut req = client.post(&url)
                .header("Content-Type", "application/x-yaml")
                .body(content);
            if let Some(k) = key {
                req = req.header("X-Api-Key", k);
            }
            match req.send() {
                Ok(r) if r.status().is_success() => {
                    let body = r.text().unwrap_or_default();
                    // Extract workflow id from response
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                            uploaded_ids.push(id.to_string());
                        }
                    }
                    println!("    {} {}", "OK".green(), wf_name);
                    upload_ok += 1;
                }
                Ok(r) => {
                    let body = r.text().unwrap_or_default();
                    println!("    {} {} — {}", "FAIL".red(), wf_name, &body[..body.len().min(80)]);
                    upload_fail += 1;
                }
                Err(e) => {
                    println!("    {} {} — {}", "FAIL".red(), wf_name, e);
                    upload_fail += 1;
                }
            }
        }

        // Activate uploaded workflows
        if activate && !uploaded_ids.is_empty() {
            println!("\n  {} Activating {} workflow(s)", "→".cyan(), uploaded_ids.len());
            for wf_id in &uploaded_ids {
                let url = format!("{}/api/admin/workflow/activate", host);
                let body = serde_json::json!({"id": wf_id, "revision": 1});
                let mut req = client.post(&url)
                    .header("Content-Type", "application/json")
                    .json(&body);
                if let Some(k) = key {
                    req = req.header("X-Api-Key", k);
                }
                match req.send() {
                    Ok(r) if r.status().is_success() => {
                        println!("    {} {}", "OK".green(), wf_id);
                    }
                    Ok(r) => {
                        println!("    {} {} — HTTP {}", "FAIL".red(), wf_id, r.status());
                    }
                    Err(e) => {
                        println!("    {} {} — {}", "FAIL".red(), wf_id, e);
                    }
                }
            }
        }
    }

    // Summary
    println!();
    println!("  {} uploaded: {}  failed: {}",
        "Upload complete.".cyan().bold(),
        format!("{}", upload_ok).green(),
        format!("{}", upload_fail).red());

    if upload_fail > 0 {
        Err(format!("{} upload(s) failed", upload_fail))
    } else {
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// Status — show provisioned inventory on server
// ═══════════════════════════════════════════════════════════════════

pub fn run_status(host: &str, key: Option<&str>, format: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;

    let get = |path: &str| -> Result<serde_json::Value, String> {
        let url = format!("{}{}", host, path);
        let mut req = client.get(&url);
        if let Some(k) = key {
            req = req.header("X-Api-Key", k);
        }
        let resp = req.send().map_err(|e| format!("GET {}: {}", path, e))?;
        if !resp.status().is_success() {
            return Err(format!("GET {} → HTTP {}", path, resp.status()));
        }
        resp.json::<serde_json::Value>().map_err(|e| format!("parse {}: {}", path, e))
    };

    let handlers = get("/api/admin/handlers")?;
    let workflows = get("/api/admin/workflows")?;

    if format == "json" {
        let combined = serde_json::json!({
            "handlers": handlers,
            "workflows": workflows,
        });
        println!("{}", serde_json::to_string_pretty(&combined).unwrap_or_default());
        return Ok(());
    }

    // ── Workflows table ──
    let wf_list = workflows.as_array()
        .or_else(|| workflows.get("workflows").and_then(|w| w.as_array()));

    println!();
    if let Some(wfs) = wf_list {
        println!("  {} ({})", "Provisioned Workflows".cyan().bold(), wfs.len());
        println!("  {:<24} {:<10} {:<30} {:<6}", "ID", "Status", "Endpoint", "Rev");
        println!("  {}", "─".repeat(74));
        for wf in wfs {
            let id = wf.get("id").and_then(|v| v.as_str()).unwrap_or("-");
            let active = wf.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
            let status_str = if active {
                "active".green().to_string()
            } else {
                "inactive".red().to_string()
            };
            let webhook = wf.get("webhook_path").and_then(|v| v.as_str()).unwrap_or("-");
            let rev = wf.get("revision").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("  {:<24} {:<10} {:<30} v{}", id, status_str, webhook, rev);
        }
    } else {
        println!("  {} (0)", "Provisioned Workflows".cyan().bold());
    }

    // ── Handlers table ──
    let plugins = handlers.get("plugins").and_then(|v| v.as_array());
    let wasm_modules = handlers.get("wasm_modules").and_then(|v| v.as_array());
    let sidecars = handlers.get("sidecars").and_then(|v| v.as_array());

    println!();
    if let Some(list) = plugins {
        println!("  {} ({})", "NativeCode Handlers".cyan().bold(), list.len());
        for p in list {
            let name = p.as_str().unwrap_or_else(|| p.get("name").and_then(|v| v.as_str()).unwrap_or("-"));
            println!("    {} {}", "✓".green(), name);
        }
    } else {
        let count = handlers.get("plugins_count").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("  {} ({})", "NativeCode Handlers".cyan().bold(), count);
    }

    println!();
    if let Some(list) = wasm_modules {
        println!("  {} ({})", "WASM Modules".cyan().bold(), list.len());
        for m in list {
            let name = m.as_str().unwrap_or_else(|| m.get("name").and_then(|v| v.as_str()).unwrap_or("-"));
            println!("    {} {}", "✓".green(), name);
        }
    } else {
        let count = handlers.get("wasm_count").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("  {} ({})", "WASM Modules".cyan().bold(), count);
    }

    println!();
    if let Some(list) = sidecars {
        println!("  {} ({})", "Sidecars".cyan().bold(), list.len());
        for s in list {
            let name = s.as_str().unwrap_or_else(|| s.get("name").and_then(|v| v.as_str()).unwrap_or("-"));
            println!("    {} {}", "✓".green(), name);
        }
    } else {
        let count = handlers.get("sidecars_count").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("  {} ({})", "Sidecars".cyan().bold(), count);
    }

    println!();
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Legacy: vflow-server provision commands
// ═══════════════════════════════════════════════════════════════════

pub enum Action {
    Push { host: String, artifact: String },
    Activate { host: String, service: String },
    Drain { host: String, service: String },
    Deactivate { host: String, service: String },
    List { host: String },
    Contract { host: String },
    Health { host: String },
}

pub fn run_provision(action: Action) -> Result<(), String> {
    match action {
        Action::Push { host, artifact } => {
            let abs_path = std::fs::canonicalize(&artifact)
                .map_err(|e| format!("Cannot find artifact '{}': {}", artifact, e))?;
            let abs_str = abs_path.to_string_lossy();

            println!("  Provisioning {} to {}", abs_str, host);
            let body = format!(r#"{{"artifact":"{}"}}"#, abs_str);

            let output = curl_post(&format!("{}/internal/provision", host), &body)?;
            println!("  {}", output);
            Ok(())
        }
        Action::Activate { host, service } => {
            println!("  Activating {} on {}", service, host);
            let output = curl_post_empty(&format!("{}/internal/activate/{}", host, service))?;
            println!("  {}", output);
            Ok(())
        }
        Action::Drain { host, service } => {
            println!("  Draining {} on {}", service, host);
            let output = curl_post_empty(&format!("{}/internal/drain/{}", host, service))?;
            println!("  {}", output);
            Ok(())
        }
        Action::Deactivate { host, service } => {
            println!("  Deactivating {} on {}", service, host);
            let output = curl_post_empty(&format!("{}/internal/deactivate/{}", host, service))?;
            println!("  {}", output);
            Ok(())
        }
        Action::List { host } => {
            let output = curl_get(&format!("{}/internal/services", host))?;
            println!("{}", output);
            Ok(())
        }
        Action::Contract { host } => {
            let output = curl_get(&format!("{}/internal/contract", host))?;
            println!("{}", output);
            Ok(())
        }
        Action::Health { host } => {
            let output = curl_get(&format!("{}/health", host))?;
            println!("{}", output);
            Ok(())
        }
    }
}

/// Upload a binary file via curl subprocess (reliable for .so/.wasm uploads).
fn upload_binary_file(url: &str, file_path: &str, header_name: &str, header_value: &str, key: Option<&str>) -> Result<String, String> {
    let mut args = vec![
        "-s".to_string(),
        "-X".to_string(), "POST".to_string(),
        "-H".to_string(), format!("{}: {}", header_name, header_value),
        "-H".to_string(), "Content-Type: application/octet-stream".to_string(),
        "--data-binary".to_string(), format!("@{}", file_path),
        "--max-time".to_string(), "30".to_string(),
    ];
    if let Some(k) = key {
        args.push("-H".to_string());
        args.push(format!("X-Api-Key: {}", k));
    }
    args.push(url.to_string());

    let output = Command::new("curl")
        .args(&args)
        .output()
        .map_err(|e| format!("curl: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl failed: {}", stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn curl_post(url: &str, body: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-s", "-X", "POST", "-H", "Content-Type: application/json", "-d", body, url])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn curl_post_empty(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-s", "-X", "POST", url])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn curl_get(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-s", url])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
