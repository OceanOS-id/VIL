// ╔════════════════════════════════════════════════════════════╗
// ║  003 — REST API Service with Transform                     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP (VilApp + ServiceProcess)               ║
// ║  Features: Zero-copy body, JSON transform, SHM context    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Minimal REST API service that receives JSON, transforms it, and returns.
// Use as a starting point — extend with your own endpoints.
//
// Run:
//   cargo run -p vil-basic-hello-server --release
//
// Test:
//   curl http://localhost:8080/api/gw/health
//   curl -X POST http://localhost:8080/api/gw/transform \
//     -H 'Content-Type: application/json' -d '{"data":"hello","value":42}'
//   curl -X POST http://localhost:8080/api/gw/echo \
//     -H 'Content-Type: application/json' -d '{"msg":"test"}'

use vil_server::prelude::*;

// ── Request / Response types ──────────────────────────────────

#[derive(Deserialize)]
struct TransformRequest {
    #[serde(default)]
    data: String,
    #[serde(default)]
    value: f64,
}

#[derive(Serialize)]
struct TransformResponse {
    original: String,
    transformed: String,
    value_doubled: f64,
    timestamp: u64,
}

#[derive(Serialize)]
struct EchoResponse {
    received_bytes: usize,
    body: serde_json::Value,
    zero_copy: bool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    shm: bool,
}

// ── Handlers ──────────────────────────────────────────────────

/// POST /transform — receive JSON, apply transformation, return result.
/// Zero-copy: body arrives via ShmSlice from ExchangeHeap.
async fn transform(body: ShmSlice) -> VilResponse<TransformResponse> {
    let req: TransformRequest = body.json().unwrap_or(TransformRequest {
        data: String::new(),
        value: 0.0,
    });

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    VilResponse::ok(TransformResponse {
        transformed: req.data.to_uppercase(),
        original: req.data,
        value_doubled: req.value * 2.0,
        timestamp,
    })
}

/// POST /echo — echo request body (useful for integration testing).
async fn echo(body: ShmSlice) -> VilResponse<EchoResponse> {
    let len = body.len();
    let json: serde_json::Value = body.json().unwrap_or(serde_json::json!(null));

    VilResponse::ok(EchoResponse {
        received_bytes: len,
        body: json,
        zero_copy: true,
    })
}

/// GET /health — service health check.
async fn health(shm: ShmContext) -> VilResponse<HealthResponse> {
    VilResponse::ok(HealthResponse {
        status: "healthy",
        service: "vil-api",
        shm: shm.available,
    })
}

#[tokio::main]
async fn main() {
    let gw = ServiceProcess::new("gw")
        .endpoint(Method::POST, "/transform", post(transform))
        .endpoint(Method::POST, "/echo", post(echo))
        .endpoint(Method::GET, "/health", get(health));

    VilApp::new("vil-basic-hello-server")
        .port(8080)
        .observer(true)
        .service(gw)
        .run()
        .await;
}
