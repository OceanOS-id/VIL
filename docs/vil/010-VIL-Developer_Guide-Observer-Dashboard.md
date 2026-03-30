# VIL Developer Guide — Part 10: Observer Dashboard & Observability

**Series:** VIL Developer Guide (10 of 10)
**Previous:** [Part 9 — Dual Architecture](./009-VIL-Developer_Guide-Dual-Architecture.md)
**Last updated:** 2026-03-30

---

## 1. Overview

VIL Observer is an embedded observability dashboard that runs inside each VIL service. Zero external dependencies, zero configuration — enable with one flag.

```bash
OBSERVER=1 ./my-vil-service
# Dashboard: http://localhost:3081/_vil/dashboard/
```

Unlike AWS CloudWatch, GCP Cloud Monitoring, or Grafana — which require separate infrastructure — VIL Observer is **in-process**, adding less than 0.5MB overhead and zero network hops for metric collection.

---

## 2. Enabling Observer

### VilApp Pattern

```rust
let app = VilApp::new("my-service")
    .port(3081)
    .observer(true)   // or read from env
    .service(svc);

app.run().await;
```

### Environment Variable

```bash
OBSERVER=1 ./target/release/my-service
OBSERVER=0 ./target/release/my-service  # disabled (default)
```

When enabled, observer serves on the **same port** as your service under the `/_vil/` prefix.

---

## 3. Dashboard Pages

### 3.1 Dashboard (Live Metrics)

The main page shows real-time service health:

| Section | Content |
|---------|---------|
| **Throughput Gauges** | Total requests, live RPS, avg RPS, success rate, slowest, memory RSS |
| **Response Time Distribution** | Fastest, Average, P95, P99, P99.9 percentile cards |
| **Req/s Live Chart** | Smooth spline canvas chart with peak indicator, 120-sample history |
| **Upstreams Table** | Per-upstream URL: requests, RPS, in-flight, latency percentiles, error rate |
| **Routes Table** | Per-route: method, path, execution class, RPS, latency breakdown, error rate |

All data refreshes automatically (configurable: 1s, 2s, 3s, 5s, 10s).

### 3.2 Topology (Service Graph)

Visual canvas rendering of service topology:

```
Client → Gateway → Routes → Upstreams
```

- Nodes auto-discovered from registered routes and upstream calls
- Color-coded: green (routes), cyan (gateway), amber (upstreams)
- Live RPS labels on each node
- Box size auto-scales to text content

### 3.3 Right Sidebar

Persistent context panel visible on all pages:

| Panel | Content |
|-------|---------|
| **SLO Budget** | Target, current rate, remaining budget, burn rate, visual bar |
| **Alerts** | Real-time threshold alerts with severity |
| **System** | PID, CPU cores, threads, FDs, memory RSS, OS/arch, uptime |
| **Config** | Profile, log level, SHM size, Rust/VIL version, health |
| **Recent Logs** | Ring buffer log stream (future: vil_log integration) |

---

## 4. SLO Budget Tracking

VIL Observer automatically tracks SLO compliance — no configuration required.

**Default target:** 99.9% success rate

### How It Works

```
Error Budget = Total Requests x (1 - Target/100)
Budget Remaining = Error Budget - Actual Errors
Burn Rate = Errors per Minute
```

### Status Levels

| Status | Condition | Action |
|--------|-----------|--------|
| **healthy** | Budget remaining > 50% | Normal operation |
| **warning** | Budget remaining > 0% but < 50% | Investigate |
| **exhausted** | Budget remaining <= 0% | SLO violated, stop deploying |

### API

```bash
curl http://localhost:3081/_vil/api/slo
```

```json
{
  "target_pct": 99.9,
  "current_pct": 99.95,
  "total_requests": 100000,
  "total_errors": 50,
  "budget_total": 100.0,
  "budget_remaining": 50.0,
  "budget_consumed_pct": 50.0,
  "burn_rate": 2.5,
  "status": "healthy"
}
```

---

## 5. Alerting

Per-node threshold alerts with automatic detection. Alerts are:
- Displayed in the sidebar Alerts panel
- Logged to stderr with `[VIL ALERT]` prefix
- Available via API for external consumers

### Built-in Thresholds

| Metric | Warning | Critical |
|--------|---------|----------|
| Error Rate | > 1% | > 5% |
| P99 Latency | > 1000ms | > 5000ms |
| Latency Spread (P99/Avg) | — | > 10x |

### API

```bash
curl http://localhost:3081/_vil/api/alerts
```

```json
{
  "alerts": [
    {
      "level": "warning",
      "metric": "error_rate",
      "message": "Error rate 2.50% exceeds 1% threshold",
      "value": "2.50%",
      "threshold": "1%"
    }
  ]
}
```

### Stdout Logging

When alerts trigger, they are logged to stderr:

```
[VIL ALERT] WARNING: Error rate 2.50% exceeds 1% threshold
[VIL ALERT] CRITICAL: POST /api/trigger p99=5200ms exceeds 5000ms
```

This integrates with any log aggregator (CloudWatch Logs, Loki, Fluentd) without additional configuration.

---

## 6. Prometheus Integration

VIL Observer exposes metrics in standard Prometheus text format, enabling integration with existing monitoring infrastructure.

### Endpoint

```
GET /_vil/metrics
Content-Type: text/plain; version=0.0.4
```

### Metrics Exposed

| Metric | Type | Description |
|--------|------|-------------|
| `vil_uptime_seconds` | gauge | Server uptime |
| `vil_requests_total` | counter | Total HTTP requests |
| `vil_errors_total` | counter | Total HTTP errors |
| `vil_memory_rss_bytes` | gauge | Resident set size |
| `vil_route_requests_total{method,path}` | counter | Per-route request count |
| `vil_route_errors_total{method,path}` | counter | Per-route error count |
| `vil_route_latency_avg_us{method,path}` | gauge | Per-route average latency |
| `vil_route_latency_p95_us{method,path}` | gauge | Per-route P95 latency |
| `vil_route_latency_p99_us{method,path}` | gauge | Per-route P99 latency |
| `vil_route_latency_p999_us{method,path}` | gauge | Per-route P99.9 latency |

### Grafana Integration

Add VIL node as a Prometheus scrape target:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'vil-services'
    scrape_interval: 5s
    static_configs:
      - targets:
        - 'node-a:3081'
        - 'node-b:3082'
        - 'node-c:3083'
    metrics_path: '/_vil/metrics'
```

Then create Grafana dashboards using standard PromQL:

```promql
# RPS per route
rate(vil_route_requests_total[1m])

# P99 latency in ms
vil_route_latency_p99_us / 1000

# Error rate
rate(vil_route_errors_total[5m]) / rate(vil_route_requests_total[5m])
```

---

## 7. API Reference

All API endpoints return JSON (except `/_vil/metrics` which returns Prometheus text format).

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/_vil/dashboard/` | GET | Embedded web dashboard (HTML) |
| `/_vil/metrics` | GET | Prometheus scrape endpoint |
| `/_vil/api/topology` | GET | App name, services, uptime, total requests |
| `/_vil/api/metrics` | GET | Per-endpoint latency/error snapshots |
| `/_vil/api/routes` | GET | Route list with percentiles and error rates |
| `/_vil/api/upstreams` | GET | Upstream call metrics |
| `/_vil/api/slo` | GET | SLO budget status |
| `/_vil/api/alerts` | GET | Current alert state |
| `/_vil/api/system` | GET | System info (PID, CPU, memory, OS) |
| `/_vil/api/config` | GET | Runtime configuration |
| `/_vil/api/shm` | GET | Shared memory ring buffer stats |
| `/_vil/api/logs/recent` | GET | Recent log entries |
| `/_vil/api/health` | GET | Health check |

---

## 8. Architecture: Single Node to Cluster

```
Phase 1 (Current): Embedded per-node
  Each VIL service has its own /_vil/ dashboard and APIs.
  Self-contained, zero external dependencies.

Phase 2 (Planned): Central dashboard
  Separate service that scrapes /_vil/metrics from all nodes.
  Aggregated view: fleet-wide SLO, cross-node topology,
  multi-instance route comparison.
```

### Per-Node (Embedded)

- Dashboard + APIs run **inside** each service process
- Metrics collected in-process (zero network hop)
- SLO/Alerts computed locally per node
- Prometheus endpoint for external scraping

### Central (Planned)

- Separate dashboard service
- Scrapes `/_vil/metrics` and `/_vil/api/*` from all nodes
- Aggregated SLO across fleet
- Cross-node topology visualization
- Fleet-wide alerting and error budget

---

## 9. Comparison with Industry Platforms

| Capability | AWS CloudWatch | GCP Monitoring | Grafana Stack | VIL Observer |
|-----------|---------------|----------------|---------------|-------------|
| Setup | Agent + config | Console + SLO defs | Prometheus + Grafana + datasources | `OBSERVER=1` |
| SLO Budget | Manual calc | Built-in (platform) | Plugin + config | Built-in (framework) |
| Per-Route Metrics | Custom metrics | Custom metrics | PromQL queries | Automatic |
| Latency Percentiles | Custom dashboard | Custom dashboard | Custom dashboard | Built-in (P50-P99.9) |
| Alerting | CloudWatch Alarms | Alert policies | Alertmanager | Built-in + stdout |
| Service Topology | X-Ray Service Map | Service Mesh | Manual config | Auto-discovered |
| Pipeline Trace | X-Ray | Cloud Trace | Tempo | SHM stage trace (planned) |
| Binary Overhead | Separate agent | Separate agent | Separate stack | ~0.5MB embedded |
| Network Overhead | Cross-network | Cross-network | Cross-network | Zero (in-process) |

---

## 10. Load Testing Integration

VIL Observer works seamlessly with external HTTP load testing tools:

```bash
# Install vastar (recommended — fast, SLO insight)
cargo install vastar

# Or use hey
wget -O hey https://storage.googleapis.com/hey-releases/hey_linux_amd64
chmod +x hey && sudo mv hey /usr/local/bin/

# Run load test while watching dashboard
vastar -n 10000 -c 500 -m POST -T "application/json" \
  -d '{"prompt":"bench"}' http://localhost:3081/api/gw/trigger

# Dashboard shows live metrics during the test
# SLO Budget updates in real-time
# Alerts fire if thresholds exceeded
```

While load test runs, open `http://localhost:3081/_vil/dashboard/` to see:
- Live RPS chart responding to load
- Latency percentiles shifting under pressure
- SLO budget consumption
- Alerts if error rate or latency spikes

---

**Previous:** [Part 9 — Dual Architecture](./009-VIL-Developer_Guide-Dual-Architecture.md)
