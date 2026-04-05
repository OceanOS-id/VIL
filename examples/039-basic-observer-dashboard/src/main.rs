// ╔════════════════════════════════════════════════════════════╗
// ║  039 — Basic Observer Dashboard                           ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   DevOps / Observability                         ║
// ║  Pattern:  VX_APP + Observer                              ║
// ║  Token:    N/A (HTTP server, not pipeline)                ║
// ║  Features: Observer dashboard, live metrics, auto-emit    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   Demonstrates VIL's embedded observer dashboard. When observer is
//   enabled, the server exposes a real-time monitoring UI at
//   `/_vil/dashboard/` and a JSON API at `/_vil/api/*`.
//
// Demonstrates:
//   - VilApp::observer(true) to enable the dashboard
//   - Built-in observer endpoints (/_vil/dashboard/, /_vil/api/*)
//   - Zero-copy request body via ShmSlice
//
// Run:
//   cargo run -p example-039-basic-observer-dashboard
//
// Test:
//   curl http://localhost:8080/api/demo/hello
//   curl http://localhost:8080/_vil/api/topology
//   curl http://localhost:8080/_vil/api/system
//   curl http://localhost:8080/_vil/api/routes
//   Open http://localhost:8080/_vil/dashboard/ in browser

use vil_server::prelude::*;

#[derive(Serialize)]
struct HelloResponse {
    message: &'static str,
    server: &'static str,
}

#[derive(Serialize)]
struct EchoResponse {
    received_bytes: usize,
    echo: serde_json::Value,
}

/// GET / — simple health probe.
async fn hello() -> VilResponse<HelloResponse> {
    VilResponse::ok(HelloResponse {
        message: "Hello from observer-enabled server!",
        server: "example-039",
    })
}

/// POST /echo — echo the request body back.
async fn echo(body: ShmSlice) -> VilResponse<EchoResponse> {
    let bytes_len = body.len();
    let json: serde_json::Value = body.json().unwrap_or(serde_json::json!(null));
    VilResponse::ok(EchoResponse {
        received_bytes: bytes_len,
        echo: json,
    })
}

#[tokio::main]
async fn main() {
    let demo_service = ServiceProcess::new("demo")
        .endpoint(Method::GET, "/hello", get(hello))
        .endpoint(Method::POST, "/echo", post(echo));

    VilApp::new("observer-demo")
        .port(8080)
        .observer(true)
        .service(demo_service)
        .run()
        .await;
}
