# VIL Observer Dashboard

Embedded real-time monitoring dashboard for VIL applications. Enable with `.observer(true)`.

## Enable Observer

```rust
// Via VilApp builder
VilApp::new("app")
    .port(8080)
    .observer(true)
    .service(service)
    .run()
    .await;

// Via VilServer builder (lower-level)
VilServer::new("app")
    .port(8080)
    .observer(true)
    .run()
    .await;
```

Via YAML manifest:
```yaml
vil_version: "6.0.0"
name: my-app
port: 8080
observer: true
```

## API Endpoints

All endpoints return JSON. Served under `/_vil/api/`:

| Endpoint | Description |
|----------|-------------|
| `GET /_vil/api/topology` | Service topology with grouped endpoint metrics |
| `GET /_vil/api/metrics` | Raw endpoint snapshots, uptime, total requests |
| `GET /_vil/api/health` | Observer health check (`{"status":"healthy"}`) |
| `GET /_vil/api/routes` | Registered routes with exec_class (fast/normal/slow/very_slow) |
| `GET /_vil/api/shm` | SHM pool stats (configured_mb, ring stripes, capacity, usage, drops) |
| `GET /_vil/api/logs/recent` | Recent resolved log events |
| `GET /_vil/api/system` | OS metrics: pid, uptime, rust_version, os, arch, cpu_count, memory_rss_kb, fd_count, thread_count |
| `GET /_vil/api/config` | Running config: profile, log_level, shm_size_mb |

Dashboard SPA: `GET /_vil/dashboard/`

## MetricsCollector

When observer is enabled, `MetricsCollector` is injected as Axum Extension:

```rust
pub struct MetricsCollector {
    endpoints: Mutex<Vec<Arc<EndpointMetrics>>>,
    started_at: Mutex<Option<Instant>>,
}

// Per-endpoint atomic counters (lock-free hot path)
pub struct EndpointMetrics {
    pub path: String,
    pub method: String,
    pub requests: AtomicU64,
    pub errors: AtomicU64,
    pub total_latency_us: AtomicU64,
    pub min_latency_us: AtomicU64,
    pub max_latency_us: AtomicU64,
}
```

## Semantic Events

`vil_observer::events` defines `#[connector_event]` types:

```rust
ObserverMetricsSnapshot { total_requests: u64, endpoint_count: u32, uptime_secs: u64, timestamp_ns: u64 }
ObserverDashboardAccess { client_hash: u32, path_hash: u32, timestamp_ns: u64 }
ObserverErrorAlert { endpoint_hash: u32, error_rate_bps: u32, request_count: u64, timestamp_ns: u64 }
```

## Auto-Emit

Observer API handlers auto-emit `system_log!`:
- Topology query: `event_type = 10`
- System info query: `event_type = 11`

## Codegen Support

When YAML has `observer: true`, codegen emits `.observer(true)` in the VilApp builder chain. All 12 YAML templates support the field via `yaml_optional_fields()`.

> Reference: docs/vil/003-VIL-Developer_Guide-Server-Framework.md §7
> Example: examples/039-basic-observer-dashboard
