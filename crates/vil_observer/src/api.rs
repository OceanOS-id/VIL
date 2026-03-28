use axum::{Router, Json, routing::get};
use axum::extract::Extension;
use crate::metrics::MetricsCollector;
use serde::Serialize;
use std::sync::Arc;
use vil_log::{system_log, types::SystemPayload};

// ── Existing types ─────────────────────────────────────────────────────────

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

// ── New types ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct RouteInfo {
    method: String,
    path: String,
    exec_class: String,
    request_count: u64,
    avg_latency_us: u64,
    p95_us: u64,
    p99_us: u64,
    p999_us: u64,
    error_rate: f64,
}

#[derive(Serialize)]
struct ShmStats {
    configured_mb: u64,
    ring_stripes: u64,
    ring_total_capacity: u64,
    ring_total_used: u64,
    ring_total_drops: u64,
}

#[derive(Serialize)]
struct LogEntry {
    timestamp_us: u64,
    level: String,
    module: String,
    message: String,
}

#[derive(Serialize)]
struct SystemInfo {
    pid: u32,
    uptime_secs: u64,
    rust_version: String,
    vil_version: String,
    os: String,
    arch: String,
    cpu_count: usize,
    memory_rss_kb: u64,
    fd_count: u64,
    thread_count: u64,
}

// ── Existing handlers ──────────────────────────────────────────────────────

async fn topology(
    Extension(collector): Extension<Arc<MetricsCollector>>,
) -> Json<TopologyResponse> {
    let start = std::time::Instant::now();
    let snapshots = collector.all_snapshots();

    // Group endpoints by service (derive from path prefix)
    let mut services_map: std::collections::HashMap<String, Vec<EndpointInfo>> =
        std::collections::HashMap::new();
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

    let _elapsed = start.elapsed();
    system_log!(Info, SystemPayload {
        event_type: 10, // observer metrics snapshot
        ..Default::default()
    });

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

// ── New handlers ───────────────────────────────────────────────────────────

/// `/_vil/api/routes` — all registered routes with details.
async fn routes(
    Extension(collector): Extension<Arc<MetricsCollector>>,
) -> Json<Vec<RouteInfo>> {
    let snapshots = collector.all_snapshots();
    let route_infos: Vec<RouteInfo> = snapshots.iter().map(|snap| {
        // Classify exec_class heuristically from avg latency
        let exec_class = if snap.avg_latency_us == 0 {
            "unknown"
        } else if snap.avg_latency_us < 1_000 {
            "fast"      // < 1 ms
        } else if snap.avg_latency_us < 10_000 {
            "normal"    // < 10 ms
        } else if snap.avg_latency_us < 100_000 {
            "slow"      // < 100 ms
        } else {
            "very_slow"
        };
        RouteInfo {
            method: snap.method.clone(),
            path: snap.path.clone(),
            exec_class: exec_class.into(),
            request_count: snap.requests,
            avg_latency_us: snap.avg_latency_us,
            p95_us: snap.p95_us,
            p99_us: snap.p99_us,
            p999_us: snap.p999_us,
            error_rate: snap.error_rate,
        }
    }).collect();
    Json(route_infos)
}

/// `/_vil/api/shm` — SHM pool stats (placeholder; real stats need vil_shm).
async fn shm_stats() -> Json<ShmStats> {
    let configured_mb = std::env::var("VIL_SHM_SIZE_MB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(64);
    Json(ShmStats {
        configured_mb,
        ring_stripes: 0,
        ring_total_capacity: 0,
        ring_total_used: 0,
        ring_total_drops: 0,
    })
}

/// `/_vil/api/logs/recent` — last N resolved log events (placeholder; needs vil_log).
async fn recent_logs() -> Json<Vec<LogEntry>> {
    Json(vec![])
}

/// `/_vil/api/system` — OS-level metrics.
async fn system_info(
    Extension(collector): Extension<Arc<MetricsCollector>>,
) -> Json<SystemInfo> {
    let start = std::time::Instant::now();
    let info = SystemInfo {
        pid: std::process::id(),
        uptime_secs: collector.uptime_secs(),
        rust_version: env!("VIL_RUST_VERSION").into(),
        vil_version: "0.2.0".into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        cpu_count: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        memory_rss_kb: read_proc_rss().unwrap_or(0),
        fd_count: read_fd_count().unwrap_or(0),
        thread_count: read_thread_count().unwrap_or(0),
    };
    let _elapsed = start.elapsed();
    system_log!(Info, SystemPayload {
        event_type: 11, // observer system info query
        ..Default::default()
    });
    Json(info)
}

/// `/_vil/api/config` — running config from environment (read-only).
async fn running_config() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "profile":      std::env::var("VIL_PROFILE").unwrap_or_else(|_| "default".into()),
        "log_level":    std::env::var("VIL_LOG_LEVEL").unwrap_or_else(|_| "info".into()),
        "shm_size_mb":  std::env::var("VIL_SHM_SIZE_MB").unwrap_or_else(|_| "64".into()),
    }))
}

// ── /proc helpers (Linux) ──────────────────────────────────────────────────

fn read_proc_rss() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status").ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
}

fn read_fd_count() -> Option<u64> {
    std::fs::read_dir("/proc/self/fd").ok()
        .map(|d| d.count() as u64)
}

fn read_thread_count() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status").ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Threads:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
}

// ── Utilities ──────────────────────────────────────────────────────────────

fn chrono_lite_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

// ── Upstreams ─────────────────────────────────────────────────────────────

/// Shared upstream snapshot data (populated by bridge task in vil_server_core).
#[derive(Clone, Default)]
pub struct UpstreamData(pub Arc<std::sync::Mutex<Vec<serde_json::Value>>>);

/// `/_vil/api/upstreams` — outbound HTTP call metrics.
async fn upstreams(
    Extension(data): Extension<UpstreamData>,
) -> Json<Vec<serde_json::Value>> {
    let snapshots = data.0.lock().unwrap().clone();
    Json(snapshots)
}

// ── Router ─────────────────────────────────────────────────────────────────

pub fn api_routes() -> Router {
    Router::new()
        .route("/_vil/api/topology",     get(topology))
        .route("/_vil/api/metrics",      get(metrics))
        .route("/_vil/api/health",       get(health))
        .route("/_vil/api/routes",       get(routes))
        .route("/_vil/api/upstreams",    get(upstreams))
        .route("/_vil/api/shm",          get(shm_stats))
        .route("/_vil/api/logs/recent",  get(recent_logs))
        .route("/_vil/api/system",       get(system_info))
        .route("/_vil/api/config",       get(running_config))
}
