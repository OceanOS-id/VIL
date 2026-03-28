// ╔════════════════════════════════════════════════════════════╗
// ║  027 — Health Check Microservice                          ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Infrastructure Monitoring / DevOps             ║
// ║  Pattern:  VIL_SERVER (simple, no ServiceProcess/VilApp)  ║
// ║  Token:    N/A                                            ║
// ║  Macros:   ShmSlice, VilResponse, #[vil_fault]            ║
// ║  Unique:   Simplest possible VIL server — 3 routes        ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   A lightweight ping/echo service for infrastructure monitoring.
//   Every production microservices deployment needs health check
//   endpoints that load balancers, Kubernetes probes, and monitoring
//   systems (Datadog, Prometheus, PagerDuty) can query to determine
//   service availability:
//
//   - /hello: simple liveness probe (is the process running?)
//   - /echo: deep health check (can it parse JSON and respond?)
//   - /health, /ready, /metrics: auto-provided by VilServer
//
//   This is the minimal VIL server pattern — no VilApp, no
//   ServiceProcess, no Tri-Lane mesh, no VX kernel. Ideal for:
//   - Sidecar health proxies in Kubernetes pods
//   - Canary endpoints for deployment verification
//   - Smoke test targets in CI/CD pipelines
//
// Why VilServer instead of VilApp?
//   VilServer is the lightweight alternative for simple APIs that
//   don't need process-oriented Tri-Lane mesh or semantic types.
//   Still gets: SHM pool, health endpoint, request tracking, tracing.
//   Use VilServer when you need a quick, reliable endpoint without
//   the overhead of service mesh participation.
//
// Run: cargo run -p vil-basic-vilserver-minimal
// Test: curl http://localhost:8080/hello
//       curl -X POST http://localhost:8080/echo -H 'Content-Type: application/json' -d '{"msg":"hi"}'

use vil_server::prelude::*;

// ApiFault defines the possible error conditions for this health service.
// Even simple monitoring endpoints benefit from typed faults — they
// enable automated alerting when probe responses indicate issues.
#[vil_fault]
pub enum ApiFault {
    InvalidInput, // Malformed health check payload
    NotFound,     // Requested probe endpoint doesn't exist
}

// Echo response for deep health checks. Monitoring systems POST a
// sample payload and verify: (1) the service parsed it correctly,
// (2) the byte count matches expectations, (3) the echo is accurate.
#[derive(Serialize)]
struct EchoResponse {
    received: usize,
    echo: serde_json::Value,
}

/// GET /hello — Liveness probe. Returns plain text to confirm the
/// service process is running and can serve HTTP requests. Load
/// balancers use this to decide whether to route traffic to this instance.
async fn hello() -> &'static str {
    "Hello from VilServer (no VX)!"
}

/// POST /echo — Deep health check via zero-copy body.
/// Monitoring systems send structured payloads and verify the echo.
/// ShmSlice ensures zero-copy access even under high probe frequency
/// (e.g., every 5 seconds from multiple monitoring sources).
async fn echo(body: ShmSlice) -> VilResponse<EchoResponse> {
    let json: serde_json::Value = body.json().unwrap_or(serde_json::json!(null));
    VilResponse::ok(EchoResponse {
        received: body.len(),
        echo: json,
    })
}

/// Note: GET /health, /ready, /metrics, /info are auto-provided
/// by VilServer — no manual registration needed. Kubernetes
/// livenessProbe and readinessProbe point to these automatically.

#[tokio::main]
async fn main() {
    // VilServer: the simplest VIL server pattern — no VilApp, no
    // ServiceProcess, no Tri-Lane mesh. Perfect for infrastructure
    // monitoring endpoints that just need to be fast and reliable.
    // Port 8080 is the standard health check port for internal services.
    VilServer::new("minimal-api")
        .port(8080)
        .route("/hello", get(hello))
        .route("/echo", post(echo))
        .run()
        .await;
}
