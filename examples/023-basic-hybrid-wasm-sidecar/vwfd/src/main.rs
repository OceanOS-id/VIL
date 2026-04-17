// 023 — Hybrid Order Pipeline (Native + WASM pricing + Sidecar fraud scoring)
// Business logic matches standard src/main.rs:
//   1. Native validation (item, qty, base_cents, customer_id)
//   2. WASM-equivalent pricing with volume discount tiers
//   3. Sidecar-equivalent fraud scoring (amount, qty, item keywords)
//   4. PPN 11% tax (1100 basis points)
use serde_json::json;

fn calculate_price(base_cents: i64, qty: i64) -> i64 {
    let subtotal = base_cents * qty;
    let discount_pct = if qty >= 100 { 20 }
        else if qty >= 50 { 10 }
        else if qty >= 10 { 5 }
        else { 0 };
    subtotal - (subtotal * discount_pct / 100)
}

fn calculate_tax(price_cents: i64) -> i64 {
    (price_cents * 1100) / 10000 // 11% PPN in basis points
}

fn score_fraud(customer_id: &str, item: &str, amount_cents: i64, qty: i64) -> (f64, &'static str, Vec<&'static str>) {
    let mut score: f64 = 0.0;
    let mut factors = Vec::new();
    let item_lower = item.to_lowercase();

    if amount_cents > 500_000 {
        score += 25.0;
        factors.push("amount_anomaly_high");
    } else if amount_cents > 200_000 {
        score += 10.0;
        factors.push("amount_anomaly_elevated");
    }
    if qty > 50 {
        score += 15.0;
        factors.push("bulk_qty_anomaly");
    }
    let risky = ["gpu", "gaming-laptop", "iphone-pro", "macbook-pro"];
    if risky.iter().any(|r| item_lower.contains(r)) {
        score += 20.0;
        factors.push("high_risk_item");
    }
    if factors.is_empty() {
        factors.push("clean");
    }

    let decision = if score > 80.0 { "BLOCK" }
        else if score > 50.0 { "REVIEW" }
        else { "PASS" };

    (score, decision, factors)
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/023-basic-hybrid-wasm-sidecar/vwfd/workflows", 8080)
        .native("hybrid_order_handler", |input| {
            let start = std::time::Instant::now();
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let item = body.get("item").and_then(|v| v.as_str()).unwrap_or("");
            let qty = body.get("qty").and_then(|v| v.as_i64()).unwrap_or(0);
            let base_cents = body.get("base_cents").and_then(|v| v.as_i64()).unwrap_or(0);
            let customer_id = body.get("customer_id").and_then(|v| v.as_str()).unwrap_or("");

            // Stage 1: Native validation
            let validate_start = std::time::Instant::now();
            if item.trim().is_empty() {
                return Ok(json!({"error": "item is required"}));
            }
            if qty <= 0 {
                return Ok(json!({"error": "qty must be > 0"}));
            }
            if base_cents <= 0 {
                return Ok(json!({"error": "base_cents must be > 0"}));
            }
            if customer_id.is_empty() {
                return Ok(json!({"error": "customer_id is required"}));
            }
            let validate_ms = validate_start.elapsed().as_secs_f64() * 1000.0;

            // Stage 2: WASM-equivalent pricing with volume discounts
            let pricing_start = std::time::Instant::now();
            let subtotal_cents = calculate_price(base_cents, qty);
            let tax_cents = calculate_tax(subtotal_cents);
            let total_cents = subtotal_cents + tax_cents;
            let pricing_ms = pricing_start.elapsed().as_secs_f64() * 1000.0;

            // Stage 3: Sidecar-equivalent fraud scoring
            let fraud_start = std::time::Instant::now();
            let (fraud_score, fraud_decision, fraud_factors) =
                score_fraud(customer_id, item, total_cents, qty);
            let fraud_ms = fraud_start.elapsed().as_secs_f64() * 1000.0;

            let total_ms = start.elapsed().as_secs_f64() * 1000.0;

            // Generate order ID
            let order_id = format!("ORD-{:05}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() % 100000);

            Ok(json!({
                "order_id": order_id,
                "item": item,
                "qty": qty,
                "subtotal_cents": subtotal_cents,
                "tax_cents": tax_cents,
                "total_cents": total_cents,
                "fraud_score": fraud_score,
                "fraud_decision": fraud_decision,
                "execution": {
                    "validate_mode": "native",
                    "pricing_mode": "wasm",
                    "fraud_mode": "sidecar",
                    "validate_ms": validate_ms,
                    "pricing_ms": pricing_ms,
                    "fraud_ms": fraud_ms,
                    "total_ms": total_ms
                }
            }))
        })
        .native("orders_health_handler", |_| {
            Ok(json!({"status": "healthy", "service": "hybrid-pipeline"}))
        })
        .run()
        .await;
}
