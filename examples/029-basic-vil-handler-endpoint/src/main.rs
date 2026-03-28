// ╔════════════════════════════════════════════════════════════╗
// ║  029 — Developer API Playground                           ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Developer Experience / API Onboarding          ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A                                            ║
// ║  Macros:   #[vil_handler], #[vil_endpoint], #[vil_fault]  ║
// ║  Unique:   Demonstrates all server macro variants          ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   Demo different handler styles for API developer onboarding. When
//   new developers join a team using VIL, this playground shows the
//   three handler patterns they can choose from:
//
//   1. Plain handler — simplest, no macros, just VilResponse
//   2. #[vil_handler] — auto RequestId + tracing + error mapping
//   3. #[vil_endpoint] — auto body extraction + execution class
//
//   This is the "developer documentation as code" pattern: instead of
//   reading docs, developers run this example and see each style in action.
//   The API playground serves as an interactive tutorial during onboarding.
//
// Why three styles?
//   Different API endpoints have different needs:
//   - Health checks: plain handler (minimal overhead)
//   - CRUD endpoints: #[vil_handler] (need request tracing)
//   - Compute endpoints: #[vil_endpoint] (need auto body extraction)
//   Developers pick the style that matches their endpoint's complexity.
//
// Run: cargo run -p vil-basic-vil-handler-endpoint
// Test:
//   curl http://localhost:8080/api/demo/plain
//   curl http://localhost:8080/api/demo/handled
//   curl -X POST http://localhost:8080/api/demo/endpoint \
//     -H 'Content-Type: application/json' -d '{"value":42}'

use vil_server::prelude::*;

// DemoFault: typed error conditions for the API playground.
// Even in a demo service, typed faults teach developers the VIL
// pattern of declaring all possible failures at compile time.
#[vil_fault]
pub enum DemoFault {
    InvalidInput,  // Malformed request (e.g., missing "value" field)
    ComputeFailed, // Computation error (e.g., overflow)
}

// ── Response types for each handler style ─────────────────────

// Plain response: minimal structure, no framework metadata.
// Used for simple status/info endpoints in API onboarding.
#[derive(Serialize)]
struct PlainResponse {
    message: String,
    style: &'static str,
}

// Handled response: includes request_id for distributed tracing.
// The #[vil_handler] macro auto-injects this for every request,
// enabling end-to-end request tracking in production systems.
#[derive(Serialize)]
struct HandledResponse {
    message: String,
    request_id: String,
    style: &'static str,
}

// Compute input/output: demonstrates typed body extraction.
// In a developer playground, this shows how VIL endpoints
// handle structured request payloads with validation.
#[derive(Deserialize)]
struct ComputeInput {
    value: u64,
}

#[derive(Serialize)]
struct ComputeOutput {
    input: u64,
    result: u64,
    style: &'static str,
}

// ── Style 1: Plain handler ──────────────────────────────────
// No macros — just ShmSlice + VilResponse (VIL Way).
// Best for: health checks, simple status endpoints, static responses.
// Developers learn this first during onboarding.

async fn plain_handler() -> VilResponse<PlainResponse> {
    VilResponse::ok(PlainResponse {
        message: "Plain handler — no macro, just VilResponse".into(),
        style: "plain",
    })
}

// ── Style 2: #[vil_handler] ─────────────────────────────────
// Auto: RequestId injection + tracing span + error mapping.
// Best for: CRUD endpoints, user-facing APIs that need audit trails.
// The macro adds observability without cluttering business logic.

// #[vil_handler] — adds RequestId + tracing (see docs)
async fn handled_handler() -> VilResponse<HandledResponse> {
    VilResponse::ok(HandledResponse {
        message: "Auto RequestId + tracing span via #[vil_handler]".into(),
        request_id: "auto-injected".into(),
        style: "vil_handler",
    })
}

// ── Style 3: #[vil_endpoint] ────────────────────────────────
// Auto: body extraction (Json wrapping for unknown types) + tracing.
// Best for: compute endpoints, data processing APIs, form handlers.
// ShmSlice gives zero-copy access to the request body for maximum
// performance in high-throughput developer API platforms.

// #[vil_endpoint] — adds auto body extraction + tracing
async fn endpoint_handler(body: ShmSlice) -> Result<VilResponse<ComputeOutput>, VilError> {
    // Zero-copy JSON parsing from ExchangeHeap — the developer
    // playground demonstrates this as the recommended pattern for
    // all POST/PUT endpoints in VIL applications.
    let input: ComputeInput = body
        .json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;
    let result = input.value * input.value; // compute square
    Ok(VilResponse::ok(ComputeOutput {
        input: input.value,
        result,
        style: "vil_endpoint",
    }))
}

#[tokio::main]
async fn main() {
    // The "demo" ServiceProcess groups all three handler styles under
    // a single /api/demo prefix. During onboarding, developers hit
    // each endpoint to see the different response shapes and features.
    let svc = ServiceProcess::new("demo")
        .endpoint(Method::GET, "/plain", get(plain_handler))
        .endpoint(Method::GET, "/handled", get(handled_handler))
        .endpoint(Method::POST, "/endpoint", post(endpoint_handler));

    // Port 8080: the developer playground service. VilApp auto-provides
    // /health and /metrics so developers can also explore VIL's
    // built-in observability during their onboarding session.
    VilApp::new("macro-demo")
        .port(8080)
        .service(svc)
        .run()
        .await;
}
