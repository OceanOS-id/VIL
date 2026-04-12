// 042 — Dynamic Pricing Rules Engine (Hybrid: WASM AssemblyScript for calculation, NativeCode for rules CRUD)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/042-basic-scripting-sandbox/vwfd/workflows", 8080)
        // Price calculation — WASM AssemblyScript (sandboxed, user-uploaded pricing logic)
        .wasm("pricing_calculate_handler", "examples/042-basic-scripting-sandbox/vwfd/wasm/assemblyscript/pricing.wasm")
        // Rules listing — NativeCode (static catalog)
        .native("pricing_rules_handler", |_| {
            Ok(json!({
                "rules": [
                    {"name": "tier_discount", "type": "percentage", "active": true},
                    {"name": "bulk_discount", "type": "threshold", "active": true},
                    {"name": "seasonal_promo", "type": "date_range", "active": false}
                ],
                "current_version": "1.0.0",
                "total_rules": 3
            }))
        })
        // Rule update — NativeCode (simple config echo)
        .native("pricing_update_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let rule = body.get("rule").and_then(|v| v.as_str()).unwrap_or("unknown");
            Ok(json!({
                "updated": true,
                "rule": rule,
                "version": "1.0.1"
            }))
        })
        .run()
        .await;
}
