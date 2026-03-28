# vil_observer

VIL Observer Dashboard — embedded web UI and JSON API for real-time service monitoring.

## Features

- **Dashboard UI** at `/_vil/dashboard/` — dark theme, 4 gauge cards, sparklines
- **8 JSON API endpoints** at `/_vil/api/*` — topology, metrics, health, routes, shm, logs, system, config
- **MetricsCollector** — atomic per-endpoint counters (requests, errors, latency min/max/avg)
- **Semantic events** — `#[connector_event]` types: `ObserverMetricsSnapshot`, `ObserverDashboardAccess`, `ObserverErrorAlert`
- **vil_log auto-emit** — `system_log!` fires on topology and system_info API calls

## Quick Start

```rust
use vil_server::prelude::*;

#[tokio::main]
async fn main() {
    let service = ServiceProcess::new("api")
        .endpoint(Method::GET, "/hello", get(|| async { "Hello!" }));

    VilApp::new("my-app")
        .port(8080)
        .observer(true)    // <-- enable observer
        .service(service)
        .run()
        .await;
}
```

Startup banner shows:
```
  Observer:     http://localhost:8080/_vil/dashboard/
```

## YAML Configuration

```yaml
vil_version: "6.0.0"
name: my-app
port: 8080
observer: true     # <-- enable observer
```

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /_vil/api/topology` | Service topology with endpoint metrics |
| `GET /_vil/api/metrics` | Raw endpoint snapshots + uptime + total requests |
| `GET /_vil/api/health` | Observer health check |
| `GET /_vil/api/routes` | Registered routes with exec_class and stats |
| `GET /_vil/api/shm` | SHM pool stats (ring stripes, capacity, usage) |
| `GET /_vil/api/logs/recent` | Recent resolved log events |
| `GET /_vil/api/system` | OS-level metrics (pid, cpu, memory, fds, threads) |
| `GET /_vil/api/config` | Running config from environment |

## Semantic Event Types

```rust
use vil_observer::events::*;

// Emitted on periodic metrics snapshot
ObserverMetricsSnapshot { total_requests, endpoint_count, uptime_secs, timestamp_ns }

// Emitted on dashboard access
ObserverDashboardAccess { client_hash, path_hash, timestamp_ns }

// Emitted when endpoint error rate exceeds threshold
ObserverErrorAlert { endpoint_hash, error_rate_bps, request_count, timestamp_ns }
```

## Part of VIL

This crate is part of [VIL](https://github.com/OceanOS-id/VIL) — a process-oriented language and framework for building zero-copy, high-performance distributed systems.

## License

Licensed under either of [Apache License 2.0](../../LICENSE-APACHE) or [MIT License](../../LICENSE-MIT).
