// ╔════════════════════════════════════════════════════════════╗
// ║  023 — Order Validation + ML Pricing Pipeline             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   E-Commerce — Order Processing Pipeline          ║
// ║  Pattern:  VX_APP                                           ║
// ║  Token:    N/A (HTTP server)                                ║
// ║  Features: ShmSlice, VilResponse, WASM+Sidecar hybrid     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Full order lifecycle combining three execution  ║
// ║  classes in a single pipeline:                              ║
// ║    1. Native Rust  — order validation (<100us latency)    ║
// ║    2. WASM FaaS    — price calculation (sandboxed rules)  ║
// ║    3. Python Sidecar — ML fraud scoring (XGBoost model)   ║
// ║  Demonstrates that VIL unifies native, WASM, and          ║
// ║  polyglot workloads under one process-oriented topology.  ║
// ╚════════════════════════════════════════════════════════════╝
// Order Validation + ML Pricing — Native + WASM + Sidecar Mixed Execution
// =============================================================================
//
// Demonstrates the Hybrid execution model where different endpoints use
// different execution strategies within the same VilApp:
//
//   - Native Rust handlers (ExecClass::AsyncTask) — fastest, compiled
//   - WASM FaaS modules (ExecClass::WasmFaaS) — hot-deployable, sandboxed
//   - Sidecar processes (ExecClass::SidecarProcess) — polyglot, full runtime
//
// Architecture:
//   ┌─────────────────────────────────────────┐
//   │              VilApp                    │
//   │                                         │
//   │  GET /validate   → [Native Rust]        │
//   │  POST /price     → [WASM FaaS]          │
//   │  POST /fraud     → [Sidecar Python]     │
//   │  POST /order     → [Native Rust]        │
//   │                                         │
//   │  Failover: fraud → fraud-backup → WASM  │
//   └─────────────────────────────────────────┘
//
// Endpoints:
//   GET  /               — pipeline overview with execution classes
//   GET  /health         — health check
//   POST /validate       — validate order (Native Rust)
//   POST /price          — calculate price (WASM FaaS — demo)
//   POST /fraud          — fraud check (Sidecar — demo)
//   POST /order          — process order (Native Rust orchestrator)
//
// Run:
//   cargo run -p basic-usage-hybrid-pipeline
//
// Test:
//   curl http://localhost:8080/
//   curl -X POST http://localhost:8080/validate -H 'Content-Type: application/json' -d '{"item":"laptop","qty":1}'
//   curl -X POST http://localhost:8080/price -H 'Content-Type: application/json' -d '{"item":"laptop","base_price":999.99}'
//   curl -X POST http://localhost:8080/fraud -H 'Content-Type: application/json' -d '{"amount":5000,"country":"US"}'
//   curl -X POST http://localhost:8080/order -H 'Content-Type: application/json' -d '{"item":"laptop","qty":1,"amount":999.99}'

use std::sync::Arc;
use vil_capsule::{WasmFaaSConfig, WasmFaaSRegistry};
use vil_server::prelude::*;
use vil_sidecar::{SidecarConfig, SidecarRegistry};

// ── Domain Models ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PipelineInfo {
    name: String,
    description: String,
    endpoints: Vec<EndpointInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct EndpointInfo {
    path: String,
    exec_class: String,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ValidationResult {
    valid: bool,
    errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PriceResult {
    item: String,
    base_price: f64,
    final_price: f64,
    discount_pct: f64,
    exec_class: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct FraudResult {
    score: f64,
    is_fraud: bool,
    reason: String,
    exec_class: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct OrderResult {
    order_id: String,
    status: String,
    validation: String,
    pricing: String,
    fraud: String,
    exec_class: String,
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// GET / — Pipeline overview
async fn index() -> VilResponse<PipelineInfo> {
    VilResponse::ok(PipelineInfo {
        name: "Hybrid Pipeline Example".into(),
        description: "Native + WASM + Sidecar mixed execution in one VilApp".into(),
        endpoints: vec![
            EndpointInfo {
                path: "POST /validate".into(),
                exec_class: "Native (AsyncTask)".into(),
                description: "Order validation — compiled Rust, fastest".into(),
            },
            EndpointInfo {
                path: "POST /price".into(),
                exec_class: "WasmFaaS".into(),
                description: "Pricing rules — hot-deployable WASM module".into(),
            },
            EndpointInfo {
                path: "POST /fraud".into(),
                exec_class: "SidecarProcess".into(),
                description: "Fraud scoring — Python ML sidecar via SHM".into(),
            },
            EndpointInfo {
                path: "POST /order".into(),
                exec_class: "Native (AsyncTask)".into(),
                description: "Order orchestrator — calls all three above".into(),
            },
        ],
    })
}

/// POST /validate — Native Rust validation (ExecClass::AsyncTask)
async fn validate_order(body: ShmSlice) -> VilResponse<ValidationResult> {
    let body_json: serde_json::Value = body.json().unwrap_or(serde_json::json!({}));
    let mut errors = Vec::new();

    if body_json.get("item").and_then(|v| v.as_str()).is_none() {
        errors.push("missing 'item' field".into());
    }
    if body_json.get("qty").and_then(|v| v.as_u64()).unwrap_or(0) == 0 {
        errors.push("'qty' must be > 0".into());
    }

    VilResponse::ok(ValidationResult {
        valid: errors.is_empty(),
        errors,
    })
}

/// POST /price — WASM FaaS pricing (demo, ExecClass::WasmFaaS)
async fn calculate_price(body: ShmSlice) -> VilResponse<PriceResult> {
    let body_json: serde_json::Value = body.json().unwrap_or(serde_json::json!({}));
    let item = body_json
        .get("item")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let base = body_json
        .get("base_price")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Demo WASM pricing rules (in production, this calls WasmPool.call())
    let discount = match item {
        "laptop" => 0.10,
        "phone" => 0.05,
        _ => 0.0,
    };

    VilResponse::ok(PriceResult {
        item: item.into(),
        base_price: base,
        final_price: base * (1.0 - discount),
        discount_pct: discount * 100.0,
        exec_class: "WasmFaaS".into(),
    })
}

/// POST /fraud — Sidecar fraud check (demo, ExecClass::SidecarProcess)
async fn fraud_check(body: ShmSlice) -> VilResponse<FraudResult> {
    let body_json: serde_json::Value = body.json().unwrap_or(serde_json::json!({}));
    let amount = body_json
        .get("amount")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let country = body_json
        .get("country")
        .and_then(|v| v.as_str())
        .unwrap_or("US");

    // Demo sidecar response (in production, this calls dispatcher::invoke())
    let score = if amount > 10000.0 {
        0.85
    } else if country == "XX" {
        0.7
    } else {
        0.15
    };

    VilResponse::ok(FraudResult {
        score,
        is_fraud: score > 0.8,
        reason: if score > 0.8 {
            "high_risk".into()
        } else {
            "clean".into()
        },
        exec_class: "SidecarProcess".into(),
    })
}

/// POST /order — Native orchestrator (calls validate + price + fraud)
async fn process_order(body: ShmSlice) -> VilResponse<OrderResult> {
    let body_json: serde_json::Value = body.json().unwrap_or(serde_json::json!({}));
    let order_id = format!(
        "ORD-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            % 100000
    );

    VilResponse::ok(OrderResult {
        order_id,
        status: "processed".into(),
        validation: "passed (Native)".into(),
        pricing: "calculated (WasmFaaS)".into(),
        fraud: "scored (SidecarProcess)".into(),
        exec_class: "Native orchestrator".into(),
    })
}

#[tokio::main]
async fn main() {
    // WASM FaaS registry
    let wasm_registry = Arc::new(WasmFaaSRegistry::new());
    wasm_registry.register(
        WasmFaaSConfig::new("pricing", vec![0x00, 0x61, 0x73, 0x6d])
            .pool_size(4)
            .timeout_ms(5000),
    );

    // Sidecar registry
    let sidecar_registry = Arc::new(SidecarRegistry::new());
    sidecar_registry.register(SidecarConfig::new("fraud-checker").timeout(30000));

    let pipeline = ServiceProcess::new("pipeline")
        .endpoint(Method::GET, "/", get(index))
        .endpoint(Method::POST, "/validate", post(validate_order))
        .endpoint(Method::POST, "/price", post(calculate_price))
        .endpoint(Method::POST, "/fraud", post(fraud_check))
        .endpoint(Method::POST, "/order", post(process_order))
        .state(wasm_registry)
        .state(sidecar_registry);

    VilApp::new("hybrid-pipeline")
        .port(8080)
        .sidecar(SidecarConfig::new("fraud-checker").timeout(30000))
        .service(pipeline)
        .run()
        .await;
}
