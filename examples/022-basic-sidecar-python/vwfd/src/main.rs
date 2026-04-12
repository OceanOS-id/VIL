// 022 — Credit Scoring (NativeCode fallback for sidecar)
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/022-basic-sidecar-python/vwfd/workflows", 8080)
        .native("credit_score_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let income = body.get("income").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let loan_amount = body.get("loan_amount").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let existing_debt = body.get("existing_debt").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let employment_years = body.get("employment_years").and_then(|v| v.as_u64()).unwrap_or(0);

            let dti_ratio = if income > 0.0 { existing_debt / income } else { 1.0 };
            let ltv_ratio = if loan_amount > 0.0 { loan_amount / (loan_amount * 1.2) } else { 1.0 };
            let base_score = 700.0 - (dti_ratio * 200.0) + (employment_years as f64 * 10.0);
            let score = (base_score.max(300.0).min(850.0)) as u32;
            let risk_class = if score >= 750 { "LOW" } else if score >= 650 { "MEDIUM" } else if score >= 550 { "HIGH" } else { "CRITICAL" };
            let recommendation = if score >= 650 { "APPROVE" } else { "DECLINE" };

            Ok(json!({
                "score": score,
                "risk_class": risk_class,
                "dti_ratio": (dti_ratio * 10000.0).round() / 10000.0,
                "ltv_ratio": (ltv_ratio * 10000.0).round() / 10000.0,
                "recommendation": recommendation,
                "factors": ["income_stability", "debt_ratio", "employment_history"]
            }))
        })
        .native("credit_health_handler", |_| {
            Ok(json!({"status": "healthy", "service": "credit-scoring"}))
        })
        .run()
        .await;
}
