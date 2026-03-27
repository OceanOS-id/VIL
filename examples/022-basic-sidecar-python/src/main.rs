// ╔════════════════════════════════════════════════════════════╗
// ║  022 — ML Fraud Scoring Sidecar                           ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   FinTech — Fraud Detection & Prevention          ║
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, ServiceCtx, VilResponse, Sidecar      ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Real-time fraud scoring for payment             ║
// ║  transactions. Python sidecar runs ML models (XGBoost,    ║
// ║  scikit-learn) trained on historical fraud patterns.       ║
// ║  Rust host manages HTTP routing and SHM transport;         ║
// ║  Python sidecar performs inference. SHM bridge avoids      ║
// ║  serialization overhead for large transaction batches.     ║
// ╚════════════════════════════════════════════════════════════╝
// ML Fraud Scoring Sidecar — Python ML Integration
// =============================================================================
//
// Demonstrates the Sidecar execution model:
//   - SidecarConfig: define sidecar with socket path, SHM size, timeout
//   - SidecarRegistry: register and manage sidecar connections
//   - VilApp::sidecar(): register sidecar config in the topology
//   - ExecClass::SidecarProcess: route endpoints to external processes
//
// The sidecar (Python fraud checker) communicates via:
//   - Unix Domain Socket (descriptors only, ~48 bytes per message)
//   - /dev/shm/vil_sc_fraud (zero-copy data via mmap)
//
// Endpoints:
//   GET  /                    — server info with sidecar status
//   GET  /health              — health check
//   POST /api/fraud/check     — fraud check (routed to Python sidecar)
//   GET  /api/fraud/status    — sidecar connection status
//
// Run:
//   # Terminal 1: Start the Rust host
//   cargo run -p basic-usage-sidecar-python
//
//   # Terminal 2: Start the Python sidecar
//   python examples-sdk/sidecar/python/fraud_checker.py
//
// Test:
//   curl http://localhost:8080/
//   curl http://localhost:8080/api/fraud/status
//   curl -X POST http://localhost:8080/api/fraud/check \
//     -H 'Content-Type: application/json' \
//     -d '{"amount": 15000, "merchant_category": "gambling", "country": "US"}'

use vil_server::prelude::*;
use vil_server::axum::extract::Extension;
use vil_sidecar::{SidecarConfig, SidecarRegistry};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ServerInfo {
    name: String,
    description: String,
    sidecars: Vec<SidecarStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct SidecarStatus {
    name: String,
    health: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct FraudResult {
    score: f64,
    is_fraud: bool,
    reason: String,
    model_version: String,
}

async fn index(
    ctx: ServiceCtx,
) -> VilResponse<ServerInfo> {
    let registry = ctx.state::<Arc<SidecarRegistry>>().expect("SidecarRegistry");
    let sidecars = registry
        .status_list()
        .into_iter()
        .map(|(name, health)| SidecarStatus {
            name,
            health: health.to_string(),
        })
        .collect();

    VilResponse::ok(ServerInfo {
        name: "Sidecar Python Example".into(),
        description: "Demonstrates Python ML sidecar with SHM zero-copy IPC".into(),
        sidecars,
    })
}

async fn fraud_status(
    ctx: ServiceCtx,
) -> VilResponse<SidecarStatus> {
    let registry = ctx.state::<Arc<SidecarRegistry>>().expect("SidecarRegistry");
    let health = registry
        .get("fraud-checker")
        .map(|e| e.health.to_string())
        .unwrap_or_else(|| "not_registered".to_string());

    VilResponse::ok(SidecarStatus {
        name: "fraud-checker".into(),
        health,
    })
}

async fn fraud_check(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> Result<VilResponse<FraudResult>, VilError> {
    let _body_json: serde_json::Value = body.json().unwrap_or(serde_json::json!({}));
    let registry = ctx.state::<Arc<SidecarRegistry>>().expect("SidecarRegistry");
    // In full integration, this would use:
    //   dispatcher::invoke(&registry, "fraud-checker", "fraud_check", &data).await
    //
    // For this example, we check sidecar status and return a demo response
    let health = registry
        .get("fraud-checker")
        .map(|e| e.health.to_string())
        .unwrap_or_else(|| "disconnected".to_string());

    if health == "healthy" {
        // Would invoke sidecar here
        Err(VilError::internal("sidecar invoke not yet wired in example"))
    } else {
        // Return fallback response when sidecar not connected
        let amount = _body_json.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let score = if amount > 10000.0 { 0.8 } else { 0.2 };

        Ok(VilResponse::ok(FraudResult {
            score,
            is_fraud: score > 0.7,
            reason: format!("fallback (sidecar {})", health),
            model_version: "fallback-v1.0".into(),
        }))
    }
}

#[tokio::main]
async fn main() {
    // Create sidecar registry
    let registry = Arc::new(SidecarRegistry::new());
    registry.register(
        SidecarConfig::new("fraud-checker")
            .command("python examples-sdk/sidecar/python/fraud_checker.py")
            .timeout(30000)
            .shm_size(64 * 1024 * 1024),
    );

    let fraud_svc = ServiceProcess::new("fraud")
        .prefix("/api/fraud")
        .endpoint(Method::GET, "/status", get(fraud_status))
        .endpoint(Method::POST, "/check", post(fraud_check))
        .state(registry.clone());

    let root_svc = ServiceProcess::new("root")
        .endpoint(Method::GET, "/", get(index))
        .state(registry.clone());

    VilApp::new("sidecar-python-example")
        .port(8080)
        .sidecar(SidecarConfig::new("fraud-checker")
            .command("python examples-sdk/sidecar/python/fraud_checker.py")
            .timeout(30000))
        .service(root_svc)
        .service(fraud_svc)
        .run()
        .await;
}
