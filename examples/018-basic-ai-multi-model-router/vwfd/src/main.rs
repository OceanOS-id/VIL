// 018-basic-ai-multi-model-router — AI Model Cost Optimizer (VWFD)
//
// Endpoints:
//   GET  /api/router/models → model list with cost info
//   POST /api/router/route  → route inference to cheapest model
//   GET  /api/router/stats  → router stats

use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static TOTAL_REQUESTS: AtomicU64 = AtomicU64::new(0);

fn list_models(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "models": [
            {"id": "gpt-4", "provider": "openai", "cost_per_1m_tokens": 30.0, "tier": "premium", "max_tokens": 8192},
            {"id": "gpt-3.5-turbo", "provider": "openai", "cost_per_1m_tokens": 2.0, "tier": "standard", "max_tokens": 4096},
            {"id": "claude-3-opus", "provider": "anthropic", "cost_per_1m_tokens": 15.0, "tier": "premium", "max_tokens": 4096}
        ]
    }))
}

fn route_inference(input: &Value) -> Result<Value, String> {
    TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);
    let prompt = input.get("body")
        .and_then(|b| b["prompt"].as_str())
        .unwrap_or("Hello");
    let max_cost = input.get("body")
        .and_then(|b| b["max_cost_usd"].as_f64())
        .unwrap_or(0.01);
    // Simple routing: use cheaper model if max_cost is low
    let (model, tier, cost) = if max_cost < 0.005 {
        ("gpt-3.5-turbo", "standard", 2.0)
    } else {
        ("gpt-4", "premium", 30.0)
    };
    Ok(json!({
        "content": format!("Response to: {}", prompt),
        "model_used": model,
        "tier": tier,
        "cost_per_1m_tokens": cost,
        "tokens_used": 42
    }))
}

fn router_stats(_input: &Value) -> Result<Value, String> {
    let total = TOTAL_REQUESTS.load(Ordering::Relaxed);
    Ok(json!({
        "total_requests": total,
        "estimated_savings_pct": 45.2,
        "models_used": {"gpt-4": 0, "gpt-3.5-turbo": 0},
        "avg_latency_ms": 120
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/018-basic-ai-multi-model-router/vwfd/workflows", 8080)
        .native("list_models", list_models)
        .native("route_inference", route_inference)
        .native("router_stats", router_stats)
        .run()
        .await;
}
