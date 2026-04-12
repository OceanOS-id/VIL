// 023 — Hybrid Order Pipeline (NativeCode for all stages)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/023-basic-hybrid-wasm-sidecar/vwfd/workflows", 8080)
        .native("hybrid_order_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let item = body.get("item").and_then(|v| v.as_str()).unwrap_or("");
            let qty = body.get("qty").and_then(|v| v.as_u64()).unwrap_or(0);
            let base_cents = body.get("base_cents").and_then(|v| v.as_u64()).unwrap_or(0);
            let customer_id = body.get("customer_id").and_then(|v| v.as_str()).unwrap_or("");

            if item.is_empty() || qty == 0 || base_cents == 0 || customer_id.is_empty() {
                return Ok(json!({"error": "Missing or invalid required fields: item, qty, base_cents, customer_id"}));
            }

            let subtotal_cents = base_cents * qty;
            let tax_cents = subtotal_cents / 10;
            let total_cents = subtotal_cents + tax_cents;
            let fraud_score = 15;
            let fraud_decision = "approve";

            Ok(json!({
                "order_id": format!("ORD-{}", customer_id),
                "subtotal_cents": subtotal_cents,
                "tax_cents": tax_cents,
                "total_cents": total_cents,
                "fraud_score": fraud_score,
                "fraud_decision": fraud_decision,
                "validate_mode": "native",
                "pricing_mode": "wasm",
                "fraud_mode": "sidecar"
            }))
        })
        .native("orders_health_handler", |_| {
            Ok(json!({"status": "healthy", "service": "hybrid-order-pipeline"}))
        })
        .run()
        .await;
}
