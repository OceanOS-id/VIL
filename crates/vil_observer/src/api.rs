use axum::{Router, Json, routing::get};
use axum::extract::Extension;
use crate::metrics::MetricsCollector;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct TopologyResponse {
    app_name: String,
    services: Vec<ServiceInfo>,
    uptime_secs: u64,
    total_requests: u64,
}

#[derive(Serialize)]
struct ServiceInfo {
    name: String,
    endpoints: Vec<EndpointInfo>,
}

#[derive(Serialize)]
struct EndpointInfo {
    method: String,
    path: String,
    requests: u64,
    error_rate: f64,
    avg_latency_us: u64,
}

async fn topology(
    Extension(collector): Extension<Arc<MetricsCollector>>,
) -> Json<TopologyResponse> {
    let snapshots = collector.all_snapshots();

    // Group endpoints by service (derive from path prefix)
    let mut services_map: std::collections::HashMap<String, Vec<EndpointInfo>> = std::collections::HashMap::new();
    for snap in &snapshots {
        let service_name = snap.path.split('/').nth(1)
            .unwrap_or("default")
            .to_string();
        services_map.entry(service_name).or_default().push(EndpointInfo {
            method: snap.method.clone(),
            path: snap.path.clone(),
            requests: snap.requests,
            error_rate: snap.error_rate,
            avg_latency_us: snap.avg_latency_us,
        });
    }

    let services: Vec<ServiceInfo> = services_map.into_iter()
        .map(|(name, endpoints)| ServiceInfo { name, endpoints })
        .collect();

    Json(TopologyResponse {
        app_name: "vil-app".into(),
        services,
        uptime_secs: collector.uptime_secs(),
        total_requests: collector.total_requests(),
    })
}

async fn metrics(
    Extension(collector): Extension<Arc<MetricsCollector>>,
) -> Json<serde_json::Value> {
    let snapshots = collector.all_snapshots();
    Json(serde_json::json!({
        "endpoints": snapshots,
        "uptime_secs": collector.uptime_secs(),
        "total_requests": collector.total_requests(),
    }))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono_lite_now(),
    }))
}

fn chrono_lite_now() -> String {
    // Simple ISO timestamp without chrono dependency
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

pub fn api_routes() -> Router {
    Router::new()
        .route("/_vil/api/topology", get(topology))
        .route("/_vil/api/metrics", get(metrics))
        .route("/_vil/api/health", get(health))
}
