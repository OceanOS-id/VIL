// 034 — Credit Risk Monte Carlo Simulation (VWFD)
// Business logic identical to standard:
//   - DTI ratio: debt / income
//   - History factor: 1 / (1 + months/120)
//   - Monte Carlo: N iterations with sin-based noise
//   - Default threshold: scenario_risk > 0.6
//   - Output: avg_score, default_probability, confidence
use serde_json::{json, Value};

fn monte_carlo_risk(income: u64, debt: u64, history_months: u32, simulations: u64) -> (f64, f64, f64) {
    let dti = debt as f64 / income.max(1) as f64;
    let history_factor = 1.0 / (1.0 + history_months as f64 / 120.0);

    let mut defaults = 0u64;
    let mut score_sum = 0.0f64;
    for i in 1..=simulations {
        let noise = ((i as f64 * 2.7182818).sin() + 1.0) / 2.0;
        let scenario_risk = dti * 0.5 + history_factor * 0.3 + noise * 0.2;
        score_sum += scenario_risk;
        if scenario_risk > 0.6 { defaults += 1; }
    }

    let avg_score = (score_sum / simulations as f64 * 1000.0).round() / 1000.0;
    let default_prob = (defaults as f64 / simulations as f64 * 10000.0).round() / 10000.0;
    let confidence = (1.0 - 1.0 / (simulations as f64).sqrt()) * 100.0;
    (avg_score, default_prob, (confidence * 100.0).round() / 100.0)
}

fn assess_risk(input: &Value) -> Result<Value, String> {
    let body = input.get("body").cloned().unwrap_or(json!({}));
    let applicant_id = body["applicant_id"].as_str().unwrap_or("unknown");
    let income = body["annual_income_cents"].as_u64().unwrap_or(0);
    let debt = body["debt_cents"].as_u64().unwrap_or(0);
    let history = body["credit_history_months"].as_u64().unwrap_or(0) as u32;
    let sims = body["simulations"].as_u64().unwrap_or(10000);

    let (avg_score, default_prob, confidence) = monte_carlo_risk(income, debt, history, sims);

    let risk_class = if default_prob < 0.05 { "LOW" }
        else if default_prob < 0.15 { "MEDIUM" }
        else if default_prob < 0.30 { "HIGH" }
        else { "CRITICAL" };

    Ok(json!({
        "applicant_id": applicant_id,
        "risk_score": avg_score,
        "default_probability": default_prob,
        "risk_class": risk_class,
        "confidence_pct": confidence,
        "simulations": sims,
        "execution_mode": "blocking_task"
    }))
}

fn risk_health(_input: &Value) -> Result<Value, String> {
    Ok(json!({"status": "healthy", "service": "risk-assessment", "mode": "monte-carlo"}))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/034-basic-blocking-task/vwfd/workflows", 8080)
        .native("assess_risk", assess_risk)
        .native("risk_health", risk_health)
        .run().await;
}
