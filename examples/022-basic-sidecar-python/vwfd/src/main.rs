// 022 — Credit Scoring (NativeCode — matches standard weighted scoring)
// Business logic matches standard src/main.rs:
//   DTI (40%): (existing_debt + loan_amount) / (income * 12)
//   LTV (30%): loan_amount / (income * 12 * 5)
//   Employment (30%): >=5yr=100, >=2yr=70, else=40
//   Combined → risk class A/B/C/D
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/022-basic-sidecar-python/vwfd/workflows", 8080)
        .native("credit_score_handler", |input| {
            let body = input.get("body").cloned().unwrap_or(json!({}));
            let nik = body.get("nik").and_then(|v| v.as_str()).unwrap_or("");
            let income = body.get("income").and_then(|v| v.as_i64()).unwrap_or(0);
            let loan_amount = body.get("loan_amount").and_then(|v| v.as_i64()).unwrap_or(0);
            let existing_debt = body.get("existing_debt").and_then(|v| v.as_i64()).unwrap_or(0);
            let employment_years = body.get("employment_years").and_then(|v| v.as_u64()).unwrap_or(0);

            let mut factors: Vec<String> = Vec::new();

            // DTI ratio (weight: 40%)
            let dti = if income > 0 {
                (existing_debt + loan_amount) as f64 / (income as f64 * 12.0)
            } else { 1.0 };
            let dti_score = if dti < 0.30 {
                factors.push("dti:excellent(<30%)".into()); 100.0
            } else if dti < 0.40 {
                factors.push(format!("dti:good({:.0}%)", dti * 100.0)); 80.0
            } else if dti < 0.50 {
                factors.push(format!("dti:moderate({:.0}%)", dti * 100.0)); 60.0
            } else {
                factors.push(format!("dti:high({:.0}%)", dti * 100.0)); 30.0
            };

            // LTV ratio (weight: 30%) — 5-year income capacity
            let ltv = if income > 0 {
                loan_amount as f64 / (income as f64 * 12.0 * 5.0)
            } else { 1.0 };
            let ltv_score = if ltv < 0.50 {
                factors.push("ltv:conservative".into()); 100.0
            } else if ltv < 0.80 {
                factors.push(format!("ltv:moderate({:.0}%)", ltv * 100.0)); 70.0
            } else {
                factors.push(format!("ltv:stretched({:.0}%)", ltv * 100.0)); 40.0
            };

            // Employment stability (weight: 30%)
            let emp_score = if employment_years >= 5 {
                factors.push(format!("employment:stable({}yr)", employment_years)); 100.0
            } else if employment_years >= 2 {
                factors.push(format!("employment:moderate({}yr)", employment_years)); 70.0
            } else {
                factors.push(format!("employment:new({}yr)", employment_years)); 40.0
            };

            // Combined weighted score
            let combined = dti_score * 0.40 + ltv_score * 0.30 + emp_score * 0.30;

            let (risk_class, recommendation) = if combined >= 80.0 {
                ("A", "Approve — low risk, standard rate")
            } else if combined >= 65.0 {
                ("B", "Approve — moderate risk, +1% premium")
            } else if combined >= 50.0 {
                ("C", "Review — borderline, require collateral")
            } else {
                ("D", "Decline — high risk, insufficient capacity")
            };

            Ok(json!({
                "nik": nik,
                "score": combined,
                "risk_class": risk_class,
                "dti_ratio": dti,
                "ltv_ratio": ltv,
                "employment_score": emp_score,
                "recommendation": recommendation,
                "factors": factors
            }))
        })
        .native("credit_health_handler", |_| {
            Ok(json!({"status": "healthy", "service": "credit-scoring"}))
        })
        .run()
        .await;
}
