// 406 — Fraud Detection: Rule-Based Parallel Scoring (VWFD)
// Business logic identical to standard:
//   POST /api/detect — 3 scorers (velocity/geo/amount) → weighted composite
//   40% velocity + 35% geo + 25% amount → APPROVE/REVIEW/BLOCK
//   GET  /api/health — service health + tools available
use serde_json::{json, Value};

fn health_handler(_input: &Value) -> Result<Value, String> {
    Ok(json!({
        "status": "ok",
        "service": "fraud-detection-agent",
        "tools_available": ["velocity_checker", "geo_analyzer", "amount_calculator"],
        "handler_modes": {
            "detect": "vwfd — workflow-driven parallel scoring",
            "health": "NativeCode — static response"
        }
    }))
}

fn geo_analyzer(input: &Value) -> Result<Value, String> {
    let country = input["country"].as_str().unwrap_or("ID");
    let city = input["city"].as_str().unwrap_or("Jakarta");
    let is_domestic = country == "ID";
    let score: u64 = if !is_domestic { 65 } else if city == "Jakarta" { 5 } else { 20 };
    Ok(json!({
        "score": score,
        "distance_km": if is_domestic { 50 } else { 8500 },
        "speed_kmh": if is_domestic { 50 } else { 950 },
        "country_changed": !is_domestic
    }))
}

fn amount_calculator(input: &Value) -> Result<Value, String> {
    let amount = input["amount_cents"].as_u64().unwrap_or(0) as f64;
    let user_mean = 15000.0_f64;
    let user_std = 8000.0_f64;
    let z = if user_std > 0.0 { (amount - user_mean) / user_std } else { 0.0 };
    let score = if z >= 4.0 { 100.0 } else if z >= 1.5 { (z - 1.5) / 2.5 * 100.0 } else { 0.0 };
    Ok(json!({
        "score": score as u64,
        "z_score": (z * 100.0).round() / 100.0,
        "user_mean": user_mean,
        "user_std": user_std,
        "current_amount": amount
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/406-agent-vil-handler-shm/vwfd/workflows", 3126)
        .sidecar("velocity_checker", "python3 examples/406-agent-vil-handler-shm/vwfd/sidecar/python/velocity_checker.py")
        .native("geo_analyzer", geo_analyzer)
        .native("amount_calculator", amount_calculator)
        .native("health_handler", health_handler)
        .run().await;
}
