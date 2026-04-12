// 031 — Banking Transaction Mesh Routing
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/031-basic-mesh-routing/vwfd/workflows", 8080)
        .native("teller_ping_handler", |_| {
            Ok(json!({
                "service": "teller",
                "teller": "online",
                "mesh_routes": ["fraud_check", "core_banking", "notification"],
                "status": "healthy"
            }))
        })
        .native("teller_submit_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let amount_cents = body.get("amount_cents").and_then(|v| v.as_u64()).unwrap_or(0);
            let fraud_score = if amount_cents > 100000 { 72 } else { 15 };
            let is_approved = fraud_score < 70;
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
            Ok(json!({
                "fraud_score": fraud_score,
                "is_approved": is_approved,
                "transaction_ref": format!("TXN-{}", ts),
                "mesh_path": ["teller", "fraud_check", "core_banking", "notification"]
            }))
        })
        .run()
        .await;
}
