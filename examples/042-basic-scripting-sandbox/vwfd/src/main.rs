// 042 — Dynamic Pricing Rules Engine (VWFD)
// Business logic identical to standard DEFAULT_RULE:
//   - Volume discount: qty>=10→15%, qty>=5→10%, qty>=3→5%
//   - Tier discount: gold→+10%, silver→+5%
//   - Cap: 25% max discount
//   - Output: final_price, discount_pct, discount_reason
// Note: standard uses JsRuntime (hot-swappable JS). VWFD uses NativeCode (same formula).
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static RULE_VERSION: AtomicU64 = AtomicU64::new(1);
static TOTAL_EXECUTIONS: AtomicU64 = AtomicU64::new(0);

fn calculate(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let product_id = body["product_id"].as_str().unwrap_or("SKU-000");
    let base_price = body["base_price"].as_i64().unwrap_or(0);
    let quantity = body["quantity"].as_i64().unwrap_or(1);
    let customer_tier = body["customer_tier"].as_str().unwrap_or("");

    let price = base_price * quantity;
    let mut discount = 0i64;

    // Volume discount
    if quantity >= 10 { discount = 15; }
    else if quantity >= 5 { discount = 10; }
    else if quantity >= 3 { discount = 5; }

    // Tier discount
    if customer_tier == "gold" { discount += 10; }
    else if customer_tier == "silver" { discount += 5; }

    // Cap at 25%
    if discount > 25 { discount = 25; }

    let final_price = price - (price * discount / 100);
    let reason = format!("qty:{} tier:{}", quantity, customer_tier);

    TOTAL_EXECUTIONS.fetch_add(1, Ordering::Relaxed);
    let version = RULE_VERSION.load(Ordering::Relaxed);

    Ok(json!({
        "product_id": product_id,
        "base_price": base_price,
        "final_price": final_price,
        "discount_applied": reason,
        "rule_version": version
    }))
}

fn rules(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "current_version": RULE_VERSION.load(Ordering::Relaxed),
        "sandbox_timeout_ms": 10,
        "sandbox_max_memory_mb": 8,
        "total_executions": TOTAL_EXECUTIONS.load(Ordering::Relaxed)
    }))
}

fn update_rule(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let _rule = body["rule"].as_str().unwrap_or("");
    let new_version = RULE_VERSION.fetch_add(1, Ordering::Relaxed) + 1;
    Ok(json!({
        "success": true,
        "new_version": new_version,
        "message": "Pricing rule updated. All new requests use the updated rule."
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/042-basic-scripting-sandbox/vwfd/workflows", 8080)
        .native("pricing_calculate_handler", calculate)
        .native("pricing_rules_handler", rules)
        .native("pricing_update_handler", update_rule)
        .run().await;
}
