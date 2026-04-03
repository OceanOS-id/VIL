// =============================================================================
// Sidecar Admin Endpoints — REST API for sidecar management
// =============================================================================
//
// Endpoints:
//   GET  /admin/sidecars              — List all sidecars with status
//   GET  /admin/sidecars/:name        — Sidecar detail (health, metrics)
//   POST /admin/sidecars/:name/drain  — Graceful drain
//   POST /admin/sidecars/:name/attach — Attach external sidecar
//   GET  /admin/sidecars/metrics      — Prometheus metrics for all sidecars
//   GET  /admin/wasm/modules          — List WASM FaaS modules

use axum::{
    extract::{Extension, Path},
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use vil_sidecar::SidecarRegistry;

/// Build the admin router for sidecar management.
///
/// Mount at `/admin` in the VilApp/VilServer.
pub fn sidecar_admin_router() -> Router {
    Router::new()
        .route("/sidecars", get(list_sidecars))
        .route("/sidecars/metrics", get(sidecar_metrics))
        .route("/sidecars/:name", get(sidecar_detail))
        .route("/sidecars/:name/drain", post(drain_sidecar))
        .route("/sidecars/:name/attach", post(attach_sidecar))
        .route("/wasm/modules", get(list_wasm_modules))
}

/// GET /admin/sidecars — List all sidecars with health status.
async fn list_sidecars(registry: Option<Extension<Arc<SidecarRegistry>>>) -> Json<Value> {
    let Some(Extension(registry)) = registry else {
        return Json(json!({
            "sidecars": [],
            "note": "no sidecar registry configured"
        }));
    };

    let sidecars: Vec<Value> = registry
        .status_list()
        .into_iter()
        .map(|(name, health)| {
            json!({
                "name": name,
                "health": health.to_string(),
            })
        })
        .collect();

    Json(json!({
        "count": sidecars.len(),
        "sidecars": sidecars,
    }))
}

/// GET /admin/sidecars/:name — Sidecar detail.
async fn sidecar_detail(
    Path(name): Path<String>,
    registry: Option<Extension<Arc<SidecarRegistry>>>,
) -> Json<Value> {
    let Some(Extension(registry)) = registry else {
        return Json(json!({"error": "no sidecar registry"}));
    };

    let detail = registry.get(&name).map(|entry| {
        let snapshot = entry.metrics.snapshot();
        json!({
            "name": name,
            "health": entry.health.to_string(),
            "methods": entry.methods.clone(),
            "config": {
                "timeout_ms": entry.config.timeout_ms,
                "shm_size": entry.config.shm_size,
                "retry": entry.config.retry,
                "pool_size": entry.config.pool_size,
            },
            "metrics": {
                "invocations": snapshot.invocations,
                "errors": snapshot.errors,
                "timeouts": snapshot.timeouts,
                "in_flight": snapshot.in_flight,
                "avg_latency_ns": snapshot.avg_latency_ns,
                "health_failures": snapshot.health_failures,
                "uptime_secs": snapshot.uptime_secs,
            },
            "has_connection": entry.connection.is_some(),
            "has_shm": entry.shm.is_some(),
            "pid": entry.pid,
        })
    });

    match detail {
        Some(val) => Json(val),
        None => Json(json!({"error": format!("sidecar '{}' not found", name)})),
    }
}

/// POST /admin/sidecars/:name/drain — Graceful drain.
async fn drain_sidecar(
    Path(name): Path<String>,
    registry: Option<Extension<Arc<SidecarRegistry>>>,
) -> Json<Value> {
    let Some(Extension(registry)) = registry else {
        return Json(json!({"error": "no sidecar registry"}));
    };

    match vil_sidecar::drain_sidecar(&registry, &name).await {
        Ok(()) => Json(json!({
            "status": "drained",
            "sidecar": name,
        })),
        Err(e) => Json(json!({
            "error": e.to_string(),
            "sidecar": name,
        })),
    }
}

/// POST /admin/sidecars/:name/attach — Attach external sidecar.
async fn attach_sidecar(
    Path(name): Path<String>,
    registry: Option<Extension<Arc<SidecarRegistry>>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let Some(Extension(registry)) = registry else {
        return Json(json!({"error": "no sidecar registry"}));
    };

    let socket = body.get("socket").and_then(|v| v.as_str()).unwrap_or("");
    if socket.is_empty() {
        return Json(json!({"error": "missing 'socket' in request body"}));
    }

    // Register with provided socket path
    let config = vil_sidecar::SidecarConfig {
        name: name.clone(),
        socket: Some(socket.to_string()),
        ..vil_sidecar::SidecarConfig::new(&name)
    };
    registry.register(config);

    Json(json!({
        "status": "registered",
        "sidecar": name,
        "socket": socket,
        "note": "call connect_sidecar() to complete handshake",
    }))
}

/// GET /admin/sidecars/metrics — Prometheus metrics.
async fn sidecar_metrics(registry: Option<Extension<Arc<SidecarRegistry>>>) -> String {
    match registry {
        Some(Extension(registry)) => registry.prometheus_metrics(),
        None => "# no sidecar registry configured\n".to_string(),
    }
}

/// GET /admin/wasm/modules — List WASM FaaS modules.
async fn list_wasm_modules() -> Json<Value> {
    // Placeholder — will be populated when VilApp integrates WasmFaaSRegistry
    Json(json!({
        "modules": [],
        "note": "WASM FaaS registry integration pending"
    }))
}
