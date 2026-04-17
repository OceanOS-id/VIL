// 031 — Banking Transaction Mesh Routing (VWFD)
// Business logic identical to standard:
//   - Fraud scoring: amount > 1_000_000 → 75, else → 12
//   - Approval threshold: fraud_score < 50
//   - Response: service, status, transaction_ref, fraud_score, is_approved
use serde_json::{json, Value};

fn teller_ping(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "service": "teller",
        "role": "Bank teller counter — submits customer transactions",
        "mesh_routes": "teller → fraud_check (Data), fraud_check → core_banking (Data), core_banking → notification (Control)"
    }))
}

fn teller_submit(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let from_account = body["from_account"].as_str().unwrap_or("");
    let to_account = body["to_account"].as_str().unwrap_or("");
    let amount_cents = body["amount_cents"].as_i64().unwrap_or(0);

    let txn_ref = format!("TXN-{}-{}", from_account, amount_cents);
    let fraud_score = if amount_cents > 1_000_000 { 75 } else { 12 };
    let is_approved = fraud_score < 50;

    Ok(json!({
        "service": "fraud_check",
        "status": if is_approved { "approved" } else { "blocked" },
        "ledger_entry_id": txn_ref,
        "from_account": from_account,
        "to_account": to_account,
        "amount_cents": amount_cents,
        "fraud_score": fraud_score,
        "is_approved": is_approved
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/031-basic-mesh-routing/vwfd/workflows", 8080)
        .native("teller_ping_handler", teller_ping)
        .native("teller_submit_handler", teller_submit)
        .run().await;
}
