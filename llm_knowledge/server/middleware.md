# Built-in Middleware

VIL server includes 21 middleware layers, auto-registered with sensible defaults.

## Default Stack

```rust
use vil_server::prelude::*;

VilApp::new("my-app")
    .port(8080)
    .service(service)
    // Middleware is auto-registered. Override specific settings:
    // .cors(CorsConfig::permissive())
    // .rate_limit(RateLimitConfig::new(1000, 60))
    .run()
    .await;
```

## Middleware Reference

### Health & Monitoring

| Middleware | Endpoint | Description |
|-----------|----------|-------------|
| Health | `GET /health` | Liveness probe |
| Readiness | `GET /ready` | Readiness with uptime |
| Metrics | `GET /metrics` | Prometheus text format |
| Observer | `GET /_vil/dashboard/` | Real-time dashboard SPA |

### Security

| Middleware | Description |
|-----------|-------------|
| JWT | Bearer token validation, configurable secret |
| RBAC | Role-based access control on endpoints |
| CSRF | Cross-site request forgery protection |
| API Key | X-Api-Key header validation |
| Rate Limiting | Per-client request rate limiting |
| IP Filter | Allow/deny by IP range |
| Circuit Breaker | Auto-trip on error threshold |

### Request Processing

| Middleware | Description |
|-----------|-------------|
| CORS | Cross-origin resource sharing |
| Compression | gzip/br response compression |
| Tracing | OpenTelemetry span injection |
| RequestId | X-Request-Id propagation |
| Timeout | Per-request timeout enforcement |
| Content Negotiation | Accept header routing |
| ETag | Conditional response caching |

### Admin Endpoints (Auto-Registered)

| Endpoint | Description |
|----------|-------------|
| `/admin/config/reload` | Hot config reload |
| `/admin/diagnostics` | Runtime diagnostics |
| `/admin/routes` | Registered routes list |
| `/admin/middleware` | Middleware introspection |
| `/admin/shm` | SHM region utilization |
| `/admin/playground` | Embedded API explorer |

## Configuration

```yaml
# vil-server.yaml
server:
  port: 8080
  request_timeout: 30

middleware:
  cors:
    allowed_origins: ["*"]
  rate_limit:
    requests_per_minute: 1000
  jwt:
    secret: "${ENV:VIL_JWT_SECRET}"
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VIL_SERVER_PORT` | 3080 | HTTP listen port |
| `VIL_METRICS_PORT` | 9090 | Prometheus port |
| `VIL_LOG_LEVEL` | info | Log level |
| `VIL_JWT_SECRET` | -- | JWT signing secret |
| `VIL_REQUEST_TIMEOUT` | 30 | Request timeout (seconds) |

> Reference: docs/vil/006-VIL-Developer_Guide-CLI-Deployment.md
