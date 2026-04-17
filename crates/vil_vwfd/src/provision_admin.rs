// =============================================================================
// vil_vwfd::provision_admin — Provisioning Admin API handlers
// =============================================================================

use std::sync::Arc;
use std::collections::HashMap;
use vil_server_core::axum::{self, extract::{Extension, Query}, response::IntoResponse, http::StatusCode, Json};
use crate::provision::WorkflowRegistry;
use crate::handler::WorkflowRouter;
use crate::plugin_loader::PluginRegistry;
use crate::app::SidecarPool;

#[cfg(feature = "wasm")]
type WasmReg = Arc<std::sync::RwLock<HashMap<String, Arc<crate::app::WasmWorkerPool>>>>;
#[cfg(not(feature = "wasm"))]
type WasmReg = Arc<std::sync::RwLock<HashMap<String, ()>>>;

fn check_auth(headers: &axum::http::HeaderMap, admin_key: &Option<String>) -> bool {
    match admin_key {
        None => true,
        Some(key) => headers.get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(|k| k == key)
            .unwrap_or(false),
    }
}

fn extract_tenant(headers: &axum::http::HeaderMap) -> String {
    headers.get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("_default")
        .to_string()
}

/// POST /api/admin/upload — Upload YAML workflow + auto-provision handlers
pub async fn upload_workflow(
    Extension(reg): Extension<Arc<WorkflowRegistry>>,
    Extension(router): Extension<Arc<WorkflowRouter>>,
    Extension(admin_key): Extension<Arc<Option<String>>>,
    Extension(plugin_reg): Extension<Arc<PluginRegistry>>,
    #[allow(unused)] Extension(wasm_reg): Extension<WasmReg>,
    Extension(sidecar_pool): Extension<Arc<std::sync::RwLock<SidecarPool>>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    if !check_auth(&headers, &admin_key) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"})));
    }
    let tenant = extract_tenant(&headers);
    let yaml = String::from_utf8_lossy(&body).to_string();

    match reg.upload(&tenant, &yaml) {
        Ok(entry) => {
            // Auto-provision handlers from .so/.wasm/sidecar
            if let Some(graph) = reg.get_graph(&tenant, &entry.id, entry.revision) {
                let provision_result = crate::handler_provision::provision_handlers(
                    &graph,
                    &plugin_reg,
                    #[cfg(feature = "wasm")]
                    &wasm_reg,
                    &sidecar_pool,
                );
                if !provision_result.provisioned.is_empty() {
                    tracing::info!("Auto-provisioned for '{}': {:?}", entry.id, provision_result.provisioned);
                }
            }

            // Auto-activate first version
            if entry.revision == 1 {
                let _ = reg.activate(&tenant, &entry.id, 1);
            }
            reg.sync_router(&router);
            (StatusCode::OK, Json(serde_json::to_value(&entry).unwrap()))
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))),
    }
}

/// POST /api/admin/upload/plugin — Upload .so plugin binary
pub async fn upload_plugin(
    Extension(admin_key): Extension<Arc<Option<String>>>,
    Extension(plugin_reg): Extension<Arc<PluginRegistry>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    if !check_auth(&headers, &admin_key) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"})));
    }

    let handler_ref = headers.get("x-handler-ref")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if handler_ref.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "X-Handler-Ref header required"})));
    }

    // Save to plugin dir — write to temp file first, then atomic rename
    // to avoid corrupting a currently-dlopen'd .so (causes SEGFAULT).
    let plugin_dir = std::env::var("VIL_PLUGIN_DIR")
        .unwrap_or_else(|_| "/var/lib/vil/plugins".to_string());
    let _ = std::fs::create_dir_all(&plugin_dir);
    let so_path = std::path::Path::new(&plugin_dir).join(format!("{}.so", handler_ref));
    let tmp_path = std::path::Path::new(&plugin_dir).join(format!(".{}.so.tmp", handler_ref));
    if let Err(e) = std::fs::write(&tmp_path, &body) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("write: {}", e)})));
    }
    if let Err(e) = std::fs::rename(&tmp_path, &so_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("rename: {}", e)})));
    }

    // Load via dlopen (safe: atomic rename preserves old inode for existing dlopen)
    match plugin_reg.load(&so_path) {
        Ok(name) => (StatusCode::OK, Json(serde_json::json!({
            "handler": name,
            "path": so_path.display().to_string(),
            "size_bytes": body.len(),
        }))),
        Err(e) => {
            let _ = std::fs::remove_file(&so_path);
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e})))
        }
    }
}

/// POST /api/admin/upload/wasm — Upload .wasm module binary
pub async fn upload_wasm(
    Extension(admin_key): Extension<Arc<Option<String>>>,
    #[allow(unused)] Extension(wasm_reg): Extension<WasmReg>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    if !check_auth(&headers, &admin_key) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"})));
    }

    let module_ref = headers.get("x-module-ref")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if module_ref.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "X-Module-Ref header required"})));
    }

    // Save to wasm dir — atomic write to avoid corrupting loaded modules
    let wasm_dir = std::env::var("VIL_WASM_DIR")
        .unwrap_or_else(|_| "/var/lib/vil/modules".to_string());
    let _ = std::fs::create_dir_all(&wasm_dir);
    let wasm_path = std::path::Path::new(&wasm_dir).join(format!("{}.wasm", module_ref));
    let tmp_path = std::path::Path::new(&wasm_dir).join(format!(".{}.wasm.tmp", module_ref));
    if let Err(e) = std::fs::write(&tmp_path, &body) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("write: {}", e)})));
    }
    if let Err(e) = std::fs::rename(&tmp_path, &wasm_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("rename: {}", e)})));
    }

    // Register via wasmtime
    #[cfg(feature = "wasm")]
    {
        match crate::handler_provision::register_wasm_from_file(&wasm_reg, &wasm_path, &module_ref) {
            Ok(()) => return (StatusCode::OK, Json(serde_json::json!({
                "module": module_ref,
                "path": wasm_path.display().to_string(),
                "size_bytes": body.len(),
            }))),
            Err(e) => {
                let _ = std::fs::remove_file(&wasm_path);
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e})));
            }
        }
    }

    #[cfg(not(feature = "wasm"))]
    {
        let _ = std::fs::remove_file(&wasm_path);
        (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "WASM feature not enabled"})))
    }
}

/// GET /api/admin/handlers — List all registered handlers
pub async fn list_handlers(
    Extension(plugin_reg): Extension<Arc<PluginRegistry>>,
    #[allow(unused)] Extension(wasm_reg): Extension<WasmReg>,
    Extension(sidecar_pool): Extension<Arc<std::sync::RwLock<SidecarPool>>>,
) -> impl IntoResponse {
    let plugins = plugin_reg.names();

    #[cfg(feature = "wasm")]
    let wasm_modules: Vec<String> = wasm_reg.read().unwrap().keys().cloned().collect();
    #[cfg(not(feature = "wasm"))]
    let wasm_modules: Vec<String> = Vec::new();

    let sidecars: Vec<String> = {
        let pool = sidecar_pool.read().unwrap();
        pool.targets()
    };

    Json(serde_json::json!({
        "plugins": plugins,
        "wasm_modules": wasm_modules,
        "sidecars": sidecars,
        "total": plugins.len() + wasm_modules.len() + sidecars.len(),
    }))
}

/// POST /api/admin/workflow/activate
pub async fn activate_workflow(
    Extension(reg): Extension<Arc<WorkflowRegistry>>,
    Extension(router): Extension<Arc<WorkflowRouter>>,
    Extension(admin_key): Extension<Arc<Option<String>>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    if !check_auth(&headers, &admin_key) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"})));
    }
    let req: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("{}", e)}))),
    };
    let tenant = req["tenant"].as_str().unwrap_or("_default");
    let id = req["id"].as_str().unwrap_or("");
    let revision = req["revision"].as_u64().unwrap_or(0) as u32;

    if id.is_empty() || revision == 0 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "id and revision required"})));
    }
    match reg.activate(tenant, id, revision) {
        Ok(entry) => {
            reg.sync_router(&router);
            (StatusCode::OK, Json(serde_json::to_value(&entry).unwrap()))
        }
        Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e}))),
    }
}

/// POST /api/admin/workflow/deactivate
pub async fn deactivate_workflow(
    Extension(reg): Extension<Arc<WorkflowRegistry>>,
    Extension(router): Extension<Arc<WorkflowRouter>>,
    Extension(admin_key): Extension<Arc<Option<String>>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    if !check_auth(&headers, &admin_key) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"})));
    }
    let req: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("{}", e)}))),
    };
    let tenant = req["tenant"].as_str().unwrap_or("_default");
    let id = req["id"].as_str().unwrap_or("");

    match reg.deactivate(tenant, id) {
        Ok(()) => {
            reg.sync_router(&router);
            (StatusCode::OK, Json(serde_json::json!({"deactivated": id})))
        }
        Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e}))),
    }
}

/// DELETE /api/admin/workflow
pub async fn remove_workflow(
    Extension(reg): Extension<Arc<WorkflowRegistry>>,
    Extension(router): Extension<Arc<WorkflowRouter>>,
    Extension(admin_key): Extension<Arc<Option<String>>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    if !check_auth(&headers, &admin_key) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"})));
    }
    let req: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("{}", e)}))),
    };
    let tenant = req["tenant"].as_str().unwrap_or("_default");
    let id = req["id"].as_str().unwrap_or("");

    if reg.remove(tenant, id) {
        reg.sync_router(&router);
        (StatusCode::OK, Json(serde_json::json!({"removed": id})))
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": format!("'{}' not found", id)})))
    }
}

/// GET /api/admin/workflows
pub async fn list_workflows(
    Extension(reg): Extension<Arc<WorkflowRegistry>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let tenant = extract_tenant(&headers);
    let t = if tenant == "_default" { None } else { Some(tenant.as_str()) };
    let workflows = reg.list(t);
    Json(serde_json::json!({"count": workflows.len(), "workflows": workflows}))
}

/// GET /api/admin/workflow/status?id=xxx
pub async fn workflow_status(
    Extension(reg): Extension<Arc<WorkflowRegistry>>,
    Query(params): Query<HashMap<String, String>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let tenant = extract_tenant(&headers);
    let id = params.get("id").cloned().unwrap_or_default();
    match reg.status(&tenant, &id) {
        Some(status) => Json(status),
        None => Json(serde_json::json!({"error": format!("'{}' not found", id)})),
    }
}

/// POST /api/admin/reload
pub async fn reload_workflows(
    Extension(reg): Extension<Arc<WorkflowRegistry>>,
    Extension(router): Extension<Arc<WorkflowRouter>>,
    Extension(admin_key): Extension<Arc<Option<String>>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if !check_auth(&headers, &admin_key) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"})));
    }
    let (loaded, errors) = reg.load_from_dir();
    reg.sync_router(&router);
    (StatusCode::OK, Json(serde_json::json!({
        "reloaded": loaded,
        "errors": errors,
    })))
}

/// GET /api/admin/health
pub async fn health(
    Extension(reg): Extension<Arc<WorkflowRegistry>>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "engine": "vil-host",
        "version": env!("CARGO_PKG_VERSION"),
        "workflows_loaded": reg.count(),
    }))
}
