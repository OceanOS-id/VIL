// 032 — Payment Gateway Failover HA (VWFD)
// Business logic identical to standard:
//   - Primary: Stripe, charge_id = ch_stripe_{amount}
//   - Backup: Adyen, charge_id = ch_adyen_{amount}
//   - Health endpoints for each gateway
use serde_json::{json, Value};

fn primary_health(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "gateway": "stripe",
        "role": "Primary payment gateway — handles all traffic by default",
        "charge_id": "n/a", "amount_cents": 0, "currency": "USD",
        "status": "healthy",
        "retry_strategy": "Retry(3) with exponential backoff before failover to backup"
    }))
}

fn primary_charge(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let amount = body["amount_cents"].as_i64().unwrap_or(0);
    let currency = body["currency"].as_str().unwrap_or("USD");
    Ok(json!({
        "gateway": "stripe",
        "role": "Primary — processed successfully",
        "charge_id": format!("ch_stripe_{}", amount),
        "amount_cents": amount, "currency": currency,
        "status": "charged",
        "retry_strategy": "Did not need retry — primary succeeded on first attempt"
    }))
}

fn backup_health(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "gateway": "adyen",
        "role": "Hot standby — activates only after primary exhausts all retries",
        "charge_id": "n/a", "amount_cents": 0, "currency": "USD",
        "status": "standby",
        "retry_strategy": "Immediate takeover — no additional retries on backup"
    }))
}

fn backup_charge(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let amount = body["amount_cents"].as_i64().unwrap_or(0);
    let currency = body["currency"].as_str().unwrap_or("USD");
    Ok(json!({
        "gateway": "adyen",
        "role": "Backup — activated after primary failover",
        "charge_id": format!("ch_adyen_{}", amount),
        "amount_cents": amount, "currency": currency,
        "status": "charged",
        "retry_strategy": "Backup does not retry — returns result immediately"
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/032-basic-failover-ha/vwfd/workflows", 8080)
        .native("primary_health", primary_health)
        .native("primary_charge", primary_charge)
        .native("backup_health", backup_health)
        .native("backup_charge", backup_charge)
        .run().await;
}
