use crate::metrics::MetricsCollector;
use axum::extract::Extension;
use axum::{routing::get, Json, Router};
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
    avg_latency_ns: u64,
}

// ── New types ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct RouteInfo {
    method: String,
    path: String,
    exec_class: String,
    request_count: u64,
    avg_latency_ns: u64,
    p95_ns: u64,
    p99_ns: u64,
    p999_ns: u64,
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
    timestamp_ns: u64,
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
        let service_name = snap.path.split('/').nth(1).unwrap_or("default").to_string();
        services_map
            .entry(service_name)
            .or_default()
            .push(EndpointInfo {
                method: snap.method.clone(),
                path: snap.path.clone(),
                requests: snap.requests,
                error_rate: snap.error_rate,
                avg_latency_ns: snap.avg_latency_ns,
            });
    }

    let services: Vec<ServiceInfo> = services_map
        .into_iter()
        .map(|(name, endpoints)| ServiceInfo { name, endpoints })
        .collect();

    let _elapsed = start.elapsed();
    system_log!(
        Info,
        SystemPayload {
            event_type: 10, // observer metrics snapshot
            ..Default::default()
        }
    );

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
async fn routes(Extension(collector): Extension<Arc<MetricsCollector>>) -> Json<Vec<RouteInfo>> {
    let snapshots = collector.all_snapshots();
    let route_infos: Vec<RouteInfo> = snapshots
        .iter()
        .map(|snap| {
            // Classify exec_class heuristically from avg latency
            let exec_class = if snap.avg_latency_ns == 0 {
                "unknown"
            } else if snap.avg_latency_ns < 1_000 {
                "fast" // < 1 ms
            } else if snap.avg_latency_ns < 10_000 {
                "normal" // < 10 ms
            } else if snap.avg_latency_ns < 100_000 {
                "slow" // < 100 ms
            } else {
                "very_slow"
            };
            RouteInfo {
                method: snap.method.clone(),
                path: snap.path.clone(),
                exec_class: exec_class.into(),
                request_count: snap.requests,
                avg_latency_ns: snap.avg_latency_ns,
                p95_ns: snap.p95_ns,
                p99_ns: snap.p99_ns,
                p999_ns: snap.p999_ns,
                error_rate: snap.error_rate,
            }
        })
        .collect();
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
async fn system_info(Extension(collector): Extension<Arc<MetricsCollector>>) -> Json<SystemInfo> {
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
    system_log!(
        Info,
        SystemPayload {
            event_type: 11, // observer system info query
            ..Default::default()
        }
    );
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
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
}

fn read_fd_count() -> Option<u64> {
    std::fs::read_dir("/proc/self/fd")
        .ok()
        .map(|d| d.count() as u64)
}

fn read_thread_count() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()
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
async fn upstreams(Extension(data): Extension<UpstreamData>) -> Json<Vec<serde_json::Value>> {
    let snapshots = data.0.lock().unwrap().clone();
    Json(snapshots)
}

// ── Prometheus Export ──────────────────────────────────────────────────────
//
// Exposes `/_vil/metrics` in Prometheus text format.
// Central dashboard / Prometheus server scrapes this endpoint from each node.

async fn prometheus_metrics(
    Extension(collector): Extension<Arc<MetricsCollector>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let snapshots = collector.all_snapshots();
    let uptime = collector.uptime_secs();
    let total_reqs = collector.total_requests();
    let total_errors: u64 = snapshots.iter().map(|s| s.errors).sum();

    let mut out = String::with_capacity(4096);

    // Global metrics
    out.push_str("# HELP vil_uptime_seconds Server uptime in seconds.\n");
    out.push_str("# TYPE vil_uptime_seconds gauge\n");
    out.push_str(&format!("vil_uptime_seconds {}\n", uptime));

    out.push_str("# HELP vil_requests_total Total HTTP requests.\n");
    out.push_str("# TYPE vil_requests_total counter\n");
    out.push_str(&format!("vil_requests_total {}\n", total_reqs));

    out.push_str("# HELP vil_errors_total Total HTTP errors.\n");
    out.push_str("# TYPE vil_errors_total counter\n");
    out.push_str(&format!("vil_errors_total {}\n", total_errors));

    // Per-route metrics
    out.push_str("# HELP vil_route_requests_total Requests per route.\n");
    out.push_str("# TYPE vil_route_requests_total counter\n");
    out.push_str("# HELP vil_route_errors_total Errors per route.\n");
    out.push_str("# TYPE vil_route_errors_total counter\n");
    out.push_str("# HELP vil_route_latency_avg_ns Average latency per route.\n");
    out.push_str("# TYPE vil_route_latency_avg_ns gauge\n");
    out.push_str("# HELP vil_route_latency_p95_ns P95 latency per route.\n");
    out.push_str("# TYPE vil_route_latency_p95_ns gauge\n");
    out.push_str("# HELP vil_route_latency_p99_ns P99 latency per route.\n");
    out.push_str("# TYPE vil_route_latency_p99_ns gauge\n");
    out.push_str("# HELP vil_route_latency_p999_ns P99.9 latency per route.\n");
    out.push_str("# TYPE vil_route_latency_p999_ns gauge\n");

    for snap in &snapshots {
        let labels = format!(
            "method=\"{}\",path=\"{}\"",
            snap.method, snap.path
        );
        out.push_str(&format!("vil_route_requests_total{{{}}} {}\n", labels, snap.requests));
        out.push_str(&format!("vil_route_errors_total{{{}}} {}\n", labels, snap.errors));
        out.push_str(&format!("vil_route_latency_avg_ns{{{}}} {}\n", labels, snap.avg_latency_ns));
        out.push_str(&format!("vil_route_latency_p95_ns{{{}}} {}\n", labels, snap.p95_ns));
        out.push_str(&format!("vil_route_latency_p99_ns{{{}}} {}\n", labels, snap.p99_ns));
        out.push_str(&format!("vil_route_latency_p999_ns{{{}}} {}\n", labels, snap.p999_ns));
    }

    // System metrics (if available via procfs)
    if let Ok(rss_kb) = get_rss_kb() {
        out.push_str("# HELP vil_memory_rss_bytes Resident set size in bytes.\n");
        out.push_str("# TYPE vil_memory_rss_bytes gauge\n");
        out.push_str(&format!("vil_memory_rss_bytes {}\n", rss_kb * 1024));
    }

    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        out,
    ).into_response()
}

fn get_rss_kb() -> Result<u64, ()> {
    // Linux: read /proc/self/status for VmRSS
    let status = std::fs::read_to_string("/proc/self/status").map_err(|_| ())?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<u64>().map_err(|_| ());
            }
        }
    }
    Err(())
}

// ── SLO Budget ────────────────────────────────────────────────────────────
//
// Per-node SLO tracking. Default target: 99.9% success rate.
// Returns current budget status, burn rate, and time-to-exhaustion.

#[derive(Serialize)]
struct SloBudget {
    target_pct: f64,
    current_pct: f64,
    total_requests: u64,
    total_errors: u64,
    budget_total: f64,
    budget_remaining: f64,
    budget_consumed_pct: f64,
    burn_rate: f64,
    status: &'static str,
}

async fn slo_budget(
    Extension(collector): Extension<Arc<MetricsCollector>>,
) -> Json<SloBudget> {
    let target = 99.9; // default SLO target
    let total_reqs = collector.total_requests();
    let snapshots = collector.all_snapshots();
    let total_errors: u64 = snapshots.iter().map(|s| s.errors).sum();
    let uptime = collector.uptime_secs().max(1);

    let current_pct = if total_reqs > 0 {
        ((total_reqs - total_errors) as f64 / total_reqs as f64) * 100.0
    } else {
        100.0
    };

    // Error budget: allowed errors = total_requests * (1 - target/100)
    let budget_total = total_reqs as f64 * (1.0 - target / 100.0);
    let budget_remaining = budget_total - total_errors as f64;
    let budget_consumed_pct = if budget_total > 0.0 {
        (total_errors as f64 / budget_total) * 100.0
    } else {
        0.0
    };

    // Burn rate: errors per minute
    let burn_rate = if uptime > 0 {
        total_errors as f64 / uptime as f64 * 60.0
    } else {
        0.0
    };

    let status = if total_reqs == 0 {
        "healthy"
    } else if budget_remaining > budget_total * 0.5 {
        "healthy"
    } else if budget_remaining > 0.0 {
        "warning"
    } else {
        "exhausted"
    };

    Json(SloBudget {
        target_pct: target,
        current_pct,
        total_requests: total_reqs,
        total_errors,
        budget_total,
        budget_remaining,
        budget_consumed_pct,
        burn_rate,
        status,
    })
}

// ── Alerting ──────────────────────────────────────────────────────────────
//
// Per-node threshold checks. Returns current alert state.
// Alerts are also logged to stdout when triggered.

#[derive(Serialize)]
struct AlertStatus {
    alerts: Vec<Alert>,
}

#[derive(Serialize)]
struct Alert {
    level: &'static str,
    metric: String,
    message: String,
    value: String,
    threshold: String,
}

async fn alert_status(
    Extension(collector): Extension<Arc<MetricsCollector>>,
) -> Json<AlertStatus> {
    let snapshots = collector.all_snapshots();
    let total_reqs = collector.total_requests();
    let total_errors: u64 = snapshots.iter().map(|s| s.errors).sum();
    let mut alerts = Vec::new();

    // Error rate alert
    let error_rate = if total_reqs > 0 {
        total_errors as f64 / total_reqs as f64
    } else { 0.0 };

    if error_rate > 0.05 {
        let msg = format!("Error rate {:.2}% exceeds 5% threshold", error_rate * 100.0);
        eprintln!("[VIL ALERT] CRITICAL: {}", msg);
        alerts.push(Alert {
            level: "critical",
            metric: "error_rate".into(),
            message: msg,
            value: format!("{:.2}%", error_rate * 100.0),
            threshold: "5%".into(),
        });
    } else if error_rate > 0.01 {
        let msg = format!("Error rate {:.2}% exceeds 1% threshold", error_rate * 100.0);
        eprintln!("[VIL ALERT] WARNING: {}", msg);
        alerts.push(Alert {
            level: "warning",
            metric: "error_rate".into(),
            message: msg,
            value: format!("{:.2}%", error_rate * 100.0),
            threshold: "1%".into(),
        });
    }

    // P99 latency alert per route
    for snap in &snapshots {
        if snap.requests < 10 { continue; } // skip low-traffic routes
        if snap.p99_ns > 5_000_000 { // > 5 seconds
            let msg = format!("{} {} p99={:.0}ms exceeds 5000ms", snap.method, snap.path, snap.p99_ns as f64 / 1000.0);
            eprintln!("[VIL ALERT] CRITICAL: {}", msg);
            alerts.push(Alert {
                level: "critical",
                metric: format!("p99_latency:{}:{}", snap.method, snap.path),
                message: msg,
                value: format!("{:.0}ms", snap.p99_ns as f64 / 1000.0),
                threshold: "5000ms".into(),
            });
        } else if snap.p99_ns > 1_000_000 { // > 1 second
            let msg = format!("{} {} p99={:.0}ms exceeds 1000ms", snap.method, snap.path, snap.p99_ns as f64 / 1000.0);
            eprintln!("[VIL ALERT] WARNING: {}", msg);
            alerts.push(Alert {
                level: "warning",
                metric: format!("p99_latency:{}:{}", snap.method, snap.path),
                message: msg,
                value: format!("{:.0}ms", snap.p99_ns as f64 / 1000.0),
                threshold: "1000ms".into(),
            });
        }

        // Spread alert: p99/p50 > 10x
        if snap.avg_latency_ns > 0 && snap.p99_ns > snap.avg_latency_ns * 10 {
            let spread = snap.p99_ns as f64 / snap.avg_latency_ns as f64;
            let msg = format!("{} {} spread p99/avg={:.1}x — high variance", snap.method, snap.path, spread);
            eprintln!("[VIL ALERT] WARNING: {}", msg);
            alerts.push(Alert {
                level: "warning",
                metric: format!("latency_spread:{}:{}", snap.method, snap.path),
                message: msg,
                value: format!("{:.1}x", spread),
                threshold: "10x".into(),
            });
        }
    }

    Json(AlertStatus { alerts })
}

// ── Router ─────────────────────────────────────────────────────────────────

pub fn api_routes() -> Router {
    Router::new()
        .route("/_vil/api/topology", get(topology))
        .route("/_vil/api/metrics", get(metrics))
        .route("/_vil/api/health", get(health))
        .route("/_vil/api/routes", get(routes))
        .route("/_vil/api/upstreams", get(upstreams))
        .route("/_vil/api/shm", get(shm_stats))
        .route("/_vil/api/logs/recent", get(recent_logs))
        .route("/_vil/api/system", get(system_info))
        .route("/_vil/api/config", get(running_config))
        .route("/_vil/metrics", get(prometheus_metrics))
        .route("/_vil/api/slo", get(slo_budget))
        .route("/_vil/api/alerts", get(alert_status))
}
