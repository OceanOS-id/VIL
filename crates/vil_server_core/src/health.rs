// =============================================================================
// VIL Server Health — Auto-registered /health, /ready, /metrics, /info
// =============================================================================

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use serde_json::json;

use crate::state::AppState;

/// Create the health router with all operational endpoints.
/// These are automatically registered by VilServer.
pub fn health_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        .route("/info", get(info_handler))
}

/// Liveness probe — always returns healthy if the server is running.
/// Used by Kubernetes liveness probes and load balancers.
async fn health_handler() -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "healthy",
            "service": "vil-server"
        })),
    )
}

/// Readiness probe — returns ready with uptime information.
/// Used by Kubernetes readiness probes to determine if the server
/// can accept traffic.
async fn ready_handler(State(state): State<AppState>) -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "ready",
            "uptime_seconds": state.uptime_secs(),
            "service": state.name()
        })),
    )
}

/// Prometheus metrics endpoint — text exposition format.
/// Scrape target for Prometheus / Grafana.
async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    // Sync runtime counters into Prometheus metrics before export
    state.sync_metrics();
    let mut body = state.metrics().to_prometheus();

    // Append per-handler metrics (zero-instrumentation observability)
    body.push_str(&state.handler_metrics().to_prometheus());
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

/// Server info endpoint — version, uptime, configuration overview.
async fn info_handler(State(state): State<AppState>) -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "name": state.name(),
            "version": state.version(),
            "uptime_seconds": state.uptime_secs(),
            "runtime": "vil-server",
            "framework": "axum",
            "vil_runtime": "VastarRuntimeWorld",
            "shm_regions": state.shm().region_count(),
            "handler_processes": state.process_registry().handler_count(),
            "tracked_routes": state.handler_metrics().route_count(),
            "rust_version": env!("CARGO_PKG_VERSION"),
        })),
    )
}
