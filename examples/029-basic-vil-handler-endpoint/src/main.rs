// ╔════════════════════════════════════════════════════════════╗
// ║  029 — VIL Handler Macro Showcase                         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Developer Experience — API Patterns             ║
// ║  Pattern:  VX_APP                                           ║
// ║  Macros:   #[vil_handler], #[vil_endpoint], #[vil_fault]  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Developer onboarding playground showing three    ║
// ║  handler styles. Macros are ACTUALLY APPLIED (not in       ║
// ║  comments). Compare responses to see what each macro adds. ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run: cargo run -p vil-basic-vil-handler-endpoint
// Test:
//   curl http://localhost:8080/api/demo/plain
//   curl http://localhost:8080/api/demo/handled
//   curl -X POST http://localhost:8080/api/demo/endpoint \
//     -H 'Content-Type: application/json' -d '{"value":42}'

use vil_server::prelude::*;
use vil_server_macros::{vil_handler, vil_endpoint};

// ── Typed fault ──────────────────────────────────────────────
#[vil_fault]
pub enum DemoFault {
    InvalidInput,
    ComputeFailed,
}

// ── Response models ──────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, VilModel)]
struct PlainResponse {
    message: String,
    style: String,
}

#[derive(Clone, Serialize, Deserialize, VilModel)]
struct HandledResponse {
    message: String,
    request_id: String,
    style: String,
}

#[derive(Deserialize)]
struct ComputeInput {
    value: u64,
}

#[derive(Clone, Serialize, Deserialize, VilModel)]
struct ComputeOutput {
    input: u64,
    result: u64,
    style: String,
}

// ── Style 1: Plain handler — no macro ────────────────────────
// Best for: health checks, simple status endpoints.

async fn plain_handler() -> VilResponse<PlainResponse> {
    VilResponse::ok(PlainResponse {
        message: "Plain handler — no macro, just VilResponse".into(),
        style: "plain".into(),
    })
}

// ── Style 2: #[vil_handler] — ACTUALLY APPLIED ──────────────
// Auto: RequestId injection + tracing span + error mapping.
// Compare response with /plain to see the difference.

#[vil_handler]
async fn handled_handler(req_id: RequestId) -> VilResponse<HandledResponse> {
    VilResponse::ok(HandledResponse {
        message: "Auto RequestId + tracing span via #[vil_handler]".into(),
        request_id: req_id.to_string(),
        style: "vil_handler".into(),
    })
}

// ── Style 3: #[vil_endpoint] — ACTUALLY APPLIED ─────────────
// Auto: body extraction + tracing + exec class dispatch.
// ShmSlice zero-copy access to request body.

#[vil_endpoint]
async fn endpoint_handler(body: ShmSlice) -> Result<VilResponse<ComputeOutput>, VilError> {
    let input: ComputeInput = body
        .json()
        .map_err(|_| VilError::bad_request("invalid JSON — expected {\"value\": N}"))?;
    let result = input.value.checked_mul(input.value)
        .ok_or_else(|| VilError::bad_request("overflow"))?;
    Ok(VilResponse::ok(ComputeOutput {
        input: input.value,
        result,
        style: "vil_endpoint".into(),
    }))
}

// ── Main ─────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let svc = ServiceProcess::new("demo")
        .endpoint(Method::GET, "/plain", get(plain_handler))
        .endpoint(Method::GET, "/handled", get(handled_handler))
        .endpoint(Method::POST, "/endpoint", post(endpoint_handler));

    VilApp::new("macro-demo")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
