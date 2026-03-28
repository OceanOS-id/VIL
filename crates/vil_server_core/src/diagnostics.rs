// =============================================================================
// VIL Server — Runtime Diagnostics
// =============================================================================
//
// Provides comprehensive runtime diagnostics at /admin/diagnostics.
// Includes: system info, runtime state, SHM status, mesh topology,
// handler registry, middleware stack, and error summary.

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use serde::Serialize;

use crate::state::AppState;

/// Create the diagnostics router.
pub fn diagnostics_router() -> Router<AppState> {
    Router::new()
        .route("/admin/diagnostics", get(diagnostics_handler))
        .route("/admin/traces", get(traces_handler))
        .route("/admin/errors", get(errors_handler))
        .route("/admin/shm", get(shm_handler))
}

async fn diagnostics_handler(State(state): State<AppState>) -> impl IntoResponse {
    let profiler = state.profiler();
    let profile = profiler.snapshot();

    let diag = DiagnosticsReport {
        server: ServerDiag {
            name: state.name().to_string(),
            version: state.version().to_string(),
            uptime_secs: state.uptime_secs(),
            pid: std::process::id(),
        },
        runtime: RuntimeDiag {
            handler_processes: state.process_registry().handler_count(),
            tracked_routes: state.handler_metrics().route_count(),
            capsule_handlers: state.capsule_registry().handler_count(),
            custom_metrics: state.custom_metrics().metric_count(),
        },
        performance: PerfDiag {
            total_requests: profile.total_requests,
            requests_per_sec: profile.requests_per_sec,
            current_connections: profile.current_connections,
            peak_connections: profile.peak_connections,
            memory_rss_bytes: profile.memory_rss_bytes,
        },
        shm: ShmDiag {
            regions: state.shm().region_count(),
            allocated_bytes: profile.shm_allocated_bytes,
            regions_created: profile.shm_regions_created,
        },
        tracing: TraceDiag {
            spans_buffered: state.span_collector().buffered(),
            spans_total: state.span_collector().total_collected(),
        },
        errors: ErrorDiag {
            tracked_errors: state.error_tracker().error_count(),
            unique_patterns: state.error_tracker().pattern_count(),
        },
    };

    axum::Json(diag)
}

async fn traces_handler(State(state): State<AppState>) -> impl IntoResponse {
    let spans = state.span_collector().recent(100);
    axum::Json(serde_json::json!({
        "spans": spans,
        "total_collected": state.span_collector().total_collected(),
        "buffered": state.span_collector().buffered(),
    }))
}

async fn errors_handler(State(state): State<AppState>) -> impl IntoResponse {
    let errors = state.error_tracker().recent(50);
    axum::Json(serde_json::json!({
        "errors": errors,
        "total": state.error_tracker().error_count(),
        "patterns": state.error_tracker().pattern_count(),
    }))
}

async fn shm_handler(State(state): State<AppState>) -> impl IntoResponse {
    let stats: Vec<serde_json::Value> = state
        .shm()
        .all_stats()
        .iter()
        .map(|s| {
            serde_json::json!({
                "capacity": s.capacity,
                "used": s.used,
                "remaining": s.remaining,
                "utilization_pct": if s.capacity > 0 { s.used * 100 / s.capacity } else { 0 },
            })
        })
        .collect();

    axum::Json(serde_json::json!({
        "region_count": state.shm().region_count(),
        "regions": stats,
    }))
}

#[derive(Serialize)]
struct DiagnosticsReport {
    server: ServerDiag,
    runtime: RuntimeDiag,
    performance: PerfDiag,
    shm: ShmDiag,
    tracing: TraceDiag,
    errors: ErrorDiag,
}

#[derive(Serialize)]
struct ServerDiag {
    name: String,
    version: String,
    uptime_secs: u64,
    pid: u32,
}

#[derive(Serialize)]
struct RuntimeDiag {
    handler_processes: usize,
    tracked_routes: usize,
    capsule_handlers: usize,
    custom_metrics: usize,
}

#[derive(Serialize)]
struct PerfDiag {
    total_requests: u64,
    requests_per_sec: f64,
    current_connections: u64,
    peak_connections: u64,
    memory_rss_bytes: u64,
}

#[derive(Serialize)]
struct ShmDiag {
    regions: usize,
    allocated_bytes: u64,
    regions_created: u64,
}

#[derive(Serialize)]
struct TraceDiag {
    spans_buffered: usize,
    spans_total: u64,
}

#[derive(Serialize)]
struct ErrorDiag {
    tracked_errors: u64,
    unique_patterns: usize,
}
