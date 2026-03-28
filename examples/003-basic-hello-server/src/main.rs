// ╔════════════════════════════════════════════════════════════╗
// ║  003 — Employee Directory Service                         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   HR / Employee Directory                        ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server, not pipeline)                ║
// ║  Macros:   #[vil_fault], VilResponse, ShmSlice, ServiceCtx║
// ║  Features: Zero-copy body, Tri-Lane context, SIMD JSON    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   A simple HR microservice that greets employees by name and echoes
//   requests for health monitoring. In an enterprise HR system, this
//   service would sit behind an API gateway and handle:
//
//   - Employee lookup and greeting (personalized onboarding screens)
//   - Request echo for health checks (load balancer probe targets)
//   - SHM diagnostics for ops teams monitoring memory utilization
//
// Why VIL Way (ShmSlice instead of Json<T>)?
//   In high-traffic HR portals (e.g., company-wide announcements, payroll
//   day spikes), zero-copy body access via ShmSlice avoids per-request
//   heap allocations. The ExchangeHeap backs all request bodies, giving
//   this service the same memory efficiency as a C service.
//
// Demonstrates VIL Way handler pattern:
//   - Zero-copy request body via ShmSlice (explicit -- VIL Way)
//   - Tri-Lane context via ServiceCtx (auto-extracted from VilApp)
//   - SIMD JSON deserialization via body.json()
//   - VilResponse for typed responses
//   - Process-oriented ServiceProcess + VilApp
//
// Run:
//   cargo run -p vil-basic-hello-server
//
// Test:
//   curl http://localhost:8080/api/hello/greet/World
//   curl -X POST http://localhost:8080/api/hello/echo \
//     -H 'Content-Type: application/json' -d '{"msg":"hi"}'
//   curl http://localhost:8080/api/hello/shm-info
//   curl http://localhost:8080/health

use vil_server::prelude::*;

// ── Response types ──────────────────────────────────────────
// Each response struct models a specific HR service capability.

// Greeting response — used when an employee accesses their personalized
// portal page. The `server` field identifies which instance served the
// request, useful for debugging in multi-replica deployments.
#[derive(Serialize)]
struct GreetResponse {
    message: String,
    server: &'static str,
}

// Echo response — used by load balancers and monitoring systems to
// verify the service is alive and correctly processing request bodies.
// The `shm_backed` flag confirms zero-copy path is active.
#[derive(Serialize)]
struct EchoResponse {
    received_bytes: usize,
    echo: serde_json::Value,
    shm_backed: bool,
}

// SHM info response — diagnostic endpoint for ops teams to monitor
// shared memory utilization. In production HR systems, SHM exhaustion
// would cause request failures during peak periods (e.g., open enrollment).
#[derive(Serialize)]
struct ShmInfoResponse {
    shm_available: bool,
    region_count: usize,
    service_name: String,
    regions: Vec<RegionStat>,
}

#[derive(Serialize)]
struct RegionStat {
    region_id: String,
    capacity: usize,
    used: usize,
    remaining: usize,
}

// ── Handlers (VIL Way) ─────────────────────────────────────
// Each handler maps to a specific HR business operation.

/// GET / — Service identity check. Load balancers and service mesh
/// sidecars hit this endpoint to verify the directory service is running.
async fn hello() -> &'static str {
    "Hello from vil-server (VIL Way)!"
}

/// GET /greet/:name — Employee greeting endpoint. In the HR portal,
/// this personalizes the welcome message when an employee logs in.
/// Path extraction provides the employee name without body parsing overhead.
async fn greet(Path(name): Path<String>) -> VilResponse<GreetResponse> {
    VilResponse::ok(GreetResponse {
        message: format!("Hello, {}!", name),
        server: "vil-server",
    })
}

/// POST /echo — Request echo for health monitoring and integration testing.
/// VIL Way: body arrives via ShmSlice (zero-copy from ExchangeHeap).
/// Developer explicitly uses ShmSlice instead of Json<T> for zero-copy access.
/// Monitoring systems POST sample payloads and verify the echo matches —
/// this catches serialization regressions before they hit real HR workflows.
async fn echo(body: ShmSlice) -> VilResponse<EchoResponse> {
    // body is ShmSlice — zero-copy bytes backed by ExchangeHeap
    let bytes_len = body.len();
    let json: serde_json::Value = body.json().unwrap_or(serde_json::json!(null));

    VilResponse::ok(EchoResponse {
        received_bytes: bytes_len,
        echo: json,
        shm_backed: true,
    })
    }

/// GET /shm-info — Shared memory diagnostics for infrastructure monitoring.
/// Uses ServiceCtx (auto-extracted) to show VIL process context.
/// Ops teams use this to monitor memory pressure during peak HR events
/// (payroll processing, annual review cycles, benefits enrollment).
async fn shm_info(
    shm: ShmContext,
    ctx: ServiceCtx,    // <- auto-extracted by VilApp (TriLaneRouter + ServiceName)
) -> VilResponse<ShmInfoResponse> {
    let regions: Vec<RegionStat> = shm
        .region_stats()
        .iter()
        .map(|s| RegionStat {
            region_id: format!("{:?}", s.region_id),
            capacity: s.capacity,
            used: s.used,
            remaining: s.remaining,
        })
        .collect();

    VilResponse::ok(ShmInfoResponse {
        shm_available: shm.available,
        region_count: shm.region_count(),
        service_name: ctx.service_name().to_string(),
        regions,
    })
}

#[tokio::main]
async fn main() {
    // VX: Define the employee directory as a named ServiceProcess.
    // The "hello" process groups all HR directory endpoints under
    // a single service boundary with shared SHM and Tri-Lane context.
    let hello_service = ServiceProcess::new("hello")
        .endpoint(Method::GET, "/", get(hello))
        .endpoint(Method::GET, "/greet/:name", get(greet))
        .endpoint(Method::POST, "/echo", post(echo))
        .endpoint(Method::GET, "/shm-info", get(shm_info));

    // VX: Run as Process-Oriented app with SHM + Tri-Lane.
    // Port 8080 is the standard internal service port. VilApp
    // auto-provides /health, /ready, /metrics for Kubernetes probes.
    VilApp::new("hello-server")
        .port(8080)
        .observer(true)
        .service(hello_service)
        .run()
        .await;
}
