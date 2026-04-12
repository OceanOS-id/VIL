// 406 — Fraud Detection (Hybrid: Python Sidecar + Go WASM + Native)
use serde_json::{json, Value};
fn amount_calculator(input: &Value) -> Result<Value, String> {
    let amount = input["amount"].as_f64().unwrap_or(0.0);
    let avg = input["historical_avg"].as_f64().unwrap_or(100.0);
    let std = input["historical_std"].as_f64().unwrap_or(50.0);
    let z = if std > 0.0 { (amount - avg) / std } else { 0.0 };
    let score = if z > 3.0 { 85 } else if z > 2.0 { 50 } else { 10 };
    Ok(json!({"amount_score": score, "z_score": format!("{:.2}", z)}))
}
#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/406-agent-vil-handler-shm/vwfd/workflows", 3126)
        .sidecar("velocity_checker", "python3 examples/406-agent-vil-handler-shm/vwfd/sidecar/python/velocity_checker.py")
        .native("geo_analyzer", |input| {
            let distance = input["distance_km"].as_f64().unwrap_or(0.0);
            let hours = input["time_hours"].as_f64().unwrap_or(24.0);
            let speed = if hours > 0.0 { distance / hours } else { 0.0 };
            let score = if speed > 900.0 { 90 } else if speed > 300.0 { 50 } else { 5 };
            Ok(serde_json::json!({"geo_score": score, "travel_speed_kmh": speed as u64}))
        })
        .native("amount_calculator", amount_calculator)
        .run().await;
}
