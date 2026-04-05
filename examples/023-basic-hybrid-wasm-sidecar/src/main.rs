// ╔════════════════════════════════════════════════════════════╗
// ║  023 — E-Commerce Order Processing Pipeline               ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   E-Commerce — Full Order Lifecycle               ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: #[vil_wasm], #[vil_sidecar], ServiceCtx,       ║
// ║            ShmSlice, VilResponse, Mixed Execution Modes    ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Single POST /order endpoint orchestrating       ║
// ║  three execution modes within one handler:                  ║
// ║    1. Native Rust  — order validation                       ║
// ║    2. #[vil_wasm]  — pricing rules (WASM sandbox)          ║
// ║    3. #[vil_sidecar] — fraud scoring (process isolation)   ║
// ║                                                             ║
// ║  NOTE: This example is pure Rust. In production, native     ║
// ║  Rust does NOT need WASM or sidecar — those add overhead.   ║
// ║  This example demonstrates the PATTERN for when you need:   ║
// ║    - WASM: sandboxed execution of untrusted/hot-deploy code ║
// ║    - Sidecar: polyglot integration (Python ML, Go service)  ║
// ║  For pure Rust, use native functions directly (fastest).    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-hybrid-wasm-sidecar
// Test:
//   curl http://localhost:8080/api/orders/health
//   curl -X POST http://localhost:8080/api/orders/order \
//     -H 'Content-Type: application/json' \
//     -d '{"item":"laptop","qty":5,"base_cents":99999,"customer_id":"C-001"}'

use vil_server::prelude::*;
use vil_server_macros::{vil_sidecar, vil_wasm};

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
struct OrderRequest {
    item: String,
    qty: i32,
    base_cents: i32,
    customer_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct OrderResult {
    order_id: String,
    item: String,
    qty: i32,
    subtotal_cents: i32,
    tax_cents: i32,
    total_cents: i32,
    fraud_score: f64,
    fraud_decision: String,
    execution: ExecutionTrace,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ExecutionTrace {
    validate_mode: String,
    pricing_mode: String,
    fraud_mode: String,
    validate_ms: f64,
    pricing_ms: f64,
    fraud_ms: f64,
    total_ms: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FraudScore {
    score: f64,
    decision: String,
    factors: Vec<String>,
}

// ── WASM Functions ───────────────────────────────────────────────────────
// Pure Rust business logic flagged for WASM sandbox execution.
// VIL auto-compiles to .wasm and manages the pool.
// In production: use this pattern when pricing rules come from
// untrusted sources or need hot-deployment without server restart.

/// Volume discount pricing engine.
/// Tiers: qty >= 100 → 20% off, >= 50 → 10%, >= 10 → 5%.
#[vil_wasm(module = "pricing")]
fn calculate_price(base_cents: i32, qty: i32) -> i32 {
    let discount = if qty >= 100 {
        20 // wholesale
    } else if qty >= 50 {
        10 // bulk
    } else if qty >= 10 {
        5 // multi-pack
    } else {
        0 // retail
    };
    let subtotal = base_cents as i64 * qty as i64;
    (subtotal - subtotal * discount as i64 / 100) as i32
}

/// Indonesian PPN tax calculation.
/// tax_bps: basis points (1100 = 11% PPN).
#[vil_wasm(module = "pricing")]
fn calculate_tax(price_cents: i32, tax_bps: i32) -> i32 {
    (price_cents as i64 * tax_bps as i64 / 10000) as i32
}

// ── Sidecar Function ─────────────────────────────────────────────────────
// Pure Rust business logic flagged for sidecar (process isolation).
// VIL auto-spawns as separate process, communicates via SHM+UDS.
// In production: use this pattern for Python ML models, Go microservices,
// or any polyglot workload that needs its own runtime.

/// Fraud scoring engine.
/// Velocity, amount anomaly, high-risk items, new customer detection.
#[vil_sidecar(target = "fraud-scorer")]
async fn score_fraud(data: &[u8]) -> FraudScore {
    let parsed: serde_json::Value = serde_json::from_slice(data).unwrap_or_default();
    let amount = parsed["amount_cents"].as_i64().unwrap_or(0);
    let qty = parsed["qty"].as_i64().unwrap_or(1);
    let item = parsed["item"].as_str().unwrap_or("");

    let mut score: f64 = 0.0;
    let mut factors = Vec::new();

    // Amount anomaly: > $5,000
    if amount > 500_000 {
        score += 25.0;
        factors.push(format!("high_amount:{}", amount));
    } else if amount > 200_000 {
        score += 10.0;
        factors.push(format!("elevated_amount:{}", amount));
    }

    // Bulk quantity anomaly
    if qty > 50 {
        score += 15.0;
        factors.push(format!("bulk_qty:{}", qty));
    }

    // High-risk item categories
    let high_risk = ["gpu", "gaming-laptop", "iphone-pro", "macbook-pro"];
    let item_lower = item.to_lowercase();
    if high_risk.iter().any(|&h| item_lower.contains(h)) {
        score += 20.0;
        factors.push(format!("high_risk_item:{}", item));
    }

    if factors.is_empty() {
        factors.push("clean".into());
    }

    let decision = if score > 80.0 {
        "BLOCK"
    } else if score > 50.0 {
        "REVIEW"
    } else {
        "PASS"
    };

    FraudScore {
        score: score.min(100.0),
        decision: decision.into(),
        factors,
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /order — Full order lifecycle.
///
/// Three execution modes in one handler — called like normal functions.
/// validate (native) → calculate_price (WASM) → score_fraud (sidecar)
async fn process_order(
    _ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<OrderResult>> {
    let start = std::time::Instant::now();
    let order: OrderRequest = body
        .json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    // ── 1. Native validation ──────────────────────────
    let v_start = std::time::Instant::now();
    if order.item.trim().is_empty() {
        return Err(VilError::bad_request("item is required"));
    }
    if order.qty <= 0 {
        return Err(VilError::bad_request("qty must be > 0"));
    }
    if order.base_cents <= 0 {
        return Err(VilError::bad_request("base_cents must be > 0"));
    }
    if order.customer_id.is_empty() {
        return Err(VilError::bad_request("customer_id is required"));
    }
    let v_ms = v_start.elapsed().as_secs_f64() * 1000.0;

    // ── 2. WASM pricing — called like normal functions ──
    let p_start = std::time::Instant::now();
    let subtotal = calculate_price(order.base_cents, order.qty);
    let tax = calculate_tax(subtotal, 1100); // 11% PPN Indonesia
    let p_ms = p_start.elapsed().as_secs_f64() * 1000.0;

    // ── 3. Sidecar fraud — called like normal async fn ──
    let f_start = std::time::Instant::now();
    let fraud_input = serde_json::json!({
        "customer_id": order.customer_id,
        "item": order.item,
        "amount_cents": subtotal + tax,
        "qty": order.qty,
    });
    let fraud = score_fraud(&serde_json::to_vec(&fraud_input).unwrap()).await;
    let f_ms = f_start.elapsed().as_secs_f64() * 1000.0;

    if fraud.decision == "BLOCK" {
        return Err(VilError::forbidden(format!(
            "blocked: score={:.0}, factors={:?}",
            fraud.score, fraud.factors
        )));
    }

    // ── 4. Native finalization ────────────────────────
    let order_id = format!(
        "ORD-{:05}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            % 100000
    );

    Ok(VilResponse::ok(OrderResult {
        order_id,
        item: order.item,
        qty: order.qty,
        subtotal_cents: subtotal,
        tax_cents: tax,
        total_cents: subtotal + tax,
        fraud_score: fraud.score,
        fraud_decision: fraud.decision,
        execution: ExecutionTrace {
            validate_mode: "native".into(),
            pricing_mode: "wasm".into(),
            fraud_mode: "sidecar".into(),
            validate_ms: v_ms,
            pricing_ms: p_ms,
            fraud_ms: f_ms,
            total_ms: start.elapsed().as_secs_f64() * 1000.0,
        },
    }))
}

/// GET /health
async fn health() -> VilResponse<serde_json::Value> {
    VilResponse::ok(serde_json::json!({
        "status": "healthy",
        "service": "hybrid-pipeline",
    }))
}

// ── Main — zero plumbing ─────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let orders = ServiceProcess::new("orders")
        .endpoint(Method::POST, "/order", post(process_order))
        .endpoint(Method::GET, "/health", get(health));

    VilApp::new("hybrid-pipeline")
        .port(8080)
        .observer(true)
        .service(orders)
        .run()
        .await;
}
