// ╔════════════════════════════════════════════════════════════╗
// ║  042 — Dynamic Pricing Rules (JS Scripting Sandbox)       ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   E-Commerce — Dynamic Pricing & Promotions       ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: JsRuntime, SandboxConfig, hot_reload,          ║
// ║            ServiceCtx, ShmSlice, VilResponse               ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Admin uploads pricing rules as JavaScript,      ║
// ║  executed in sandboxed runtime with memory+time limits.    ║
// ║  Rules can be hot-swapped without server restart.          ║
// ║  Use case: flash sale rules, loyalty discounts, geo-pricing║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-basic-scripting-sandbox
// Test:
//   curl http://localhost:8080/api/pricing/rules
//   curl -X POST http://localhost:8080/api/pricing/calculate \
//     -H 'Content-Type: application/json' \
//     -d '{"product_id":"SKU-001","base_price":100000,"quantity":3,"customer_tier":"gold"}'
//   curl -X POST http://localhost:8080/api/pricing/update-rule \
//     -H 'Content-Type: application/json' \
//     -d '{"rule":"function calculate(input) { return { final_price: input.base_price * 0.8 }; }"}'

use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};

use vil_script_js::{JsRuntime, SandboxConfig};
use vil_server::prelude::*;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct PriceRequest {
    product_id: String,
    base_price: i64,
    quantity: i32,
    #[serde(default)]
    customer_tier: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PriceResponse {
    product_id: String,
    base_price: i64,
    final_price: i64,
    discount_applied: String,
    rule_version: u64,
}

#[derive(Debug, Deserialize)]
struct UpdateRuleRequest {
    rule: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct RuleInfo {
    current_version: u64,
    sandbox_timeout_ms: u64,
    sandbox_max_memory_mb: u64,
    total_executions: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct UpdateResult {
    success: bool,
    new_version: u64,
    message: String,
}

// ── State ────────────────────────────────────────────────────────────────

struct PricingState {
    runtime: RwLock<JsRuntime>,
    executions: AtomicU64,
}

// Default pricing rule
const DEFAULT_RULE: &str = r#"
function calculate(input) {
    var price = input.base_price * input.quantity;
    var discount = 0;

    // Volume discount
    if (input.quantity >= 10) discount = 15;
    else if (input.quantity >= 5) discount = 10;
    else if (input.quantity >= 3) discount = 5;

    // Tier discount
    if (input.customer_tier === 'gold') discount += 10;
    else if (input.customer_tier === 'silver') discount += 5;

    // Cap discount at 25%
    if (discount > 25) discount = 25;

    var final_price = price - (price * discount / 100);
    return {
        final_price: Math.round(final_price),
        discount_pct: discount,
        discount_reason: 'qty:' + input.quantity + ' tier:' + input.customer_tier
    };
}
"#;

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /calculate — Execute pricing rule in JS sandbox.
async fn calculate(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<PriceResponse>> {
    let req: PriceRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    if req.base_price <= 0 {
        return Err(VilError::bad_request("base_price must be positive"));
    }

    let state = ctx.state::<Arc<PricingState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    state.executions.fetch_add(1, Ordering::Relaxed);

    // Prepare input for JS runtime
    let input = serde_json::json!({
        "product_id": req.product_id,
        "base_price": req.base_price,
        "quantity": req.quantity,
        "customer_tier": req.customer_tier,
    });

    // Execute in sandbox (memory + time limited)
    let result = {
        let runtime = state.runtime.read().unwrap();
        runtime.execute(input)
            .map_err(|e| VilError::internal(format!("script execution failed: {}", e)))?
    };

    let final_price = result["final_price"].as_i64().unwrap_or(req.base_price);
    let discount_reason = result["discount_reason"].as_str().unwrap_or("none");
    let version = state.runtime.read().unwrap().version();

    Ok(VilResponse::ok(PriceResponse {
        product_id: req.product_id,
        base_price: req.base_price,
        final_price,
        discount_applied: discount_reason.into(),
        rule_version: version,
    }))
}

/// POST /update-rule — Hot-swap pricing rule without restart.
async fn update_rule(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<UpdateResult>> {
    let req: UpdateRuleRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    let state = ctx.state::<Arc<PricingState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let new_version = {
        let mut runtime = state.runtime.write().unwrap();
        runtime.load_inline(&req.rule);
        runtime.hot_reload()
            .map_err(|e| VilError::bad_request(format!("invalid script: {}", e)))?
    };

    Ok(VilResponse::ok(UpdateResult {
        success: true,
        new_version,
        message: "Pricing rule updated. All new requests use the updated rule.".into(),
    }))
}

/// GET /rules — Current rule info + execution stats.
async fn rules(ctx: ServiceCtx) -> HandlerResult<VilResponse<RuleInfo>> {
    let state = ctx.state::<Arc<PricingState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let version = state.runtime.read().unwrap().version();

    Ok(VilResponse::ok(RuleInfo {
        current_version: version,
        sandbox_timeout_ms: 10,
        sandbox_max_memory_mb: 8,
        total_executions: state.executions.load(Ordering::Relaxed),
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // Initialize JS sandbox with strict limits
    let sandbox_cfg = SandboxConfig {
        timeout_ms: 10,        // 10ms max per execution
        max_memory_mb: 8,      // 8MB memory limit
        allow_net: false,      // no network access
        allow_fs: false,       // no filesystem access
        max_output_size_kb: 64,
    };

    let mut runtime = JsRuntime::new(sandbox_cfg);
    runtime.load_inline(DEFAULT_RULE);

    let state = Arc::new(PricingState {
        runtime: RwLock::new(runtime),
        executions: AtomicU64::new(0),
    });

    let svc = ServiceProcess::new("pricing")
        .endpoint(Method::POST, "/calculate", post(calculate))
        .endpoint(Method::POST, "/update-rule", post(update_rule))
        .endpoint(Method::GET, "/rules", get(rules))
        .state(state);

    VilApp::new("dynamic-pricing-engine")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
