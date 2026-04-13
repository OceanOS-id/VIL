// =============================================================================
// vil_vwfd::provision_admin — Provisioning Admin API handlers
// =============================================================================

use std::sync::Arc;
use std::collections::HashMap;
use vil_server_core::axum::{self, extract::{Extension, Query}, response::IntoResponse, http::StatusCode, Json};
use crate::provision::WorkflowRegistry;
use crate::handler::WorkflowRouter;

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

/// POST /api/admin/upload
pub async fn upload_workflow(
    Extension(reg): Extension<Arc<WorkflowRegistry>>,
    Extension(router): Extension<Arc<WorkflowRouter>>,
    Extension(admin_key): Extension<Arc<Option<String>>>,
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
