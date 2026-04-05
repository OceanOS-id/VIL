// ╔════════════════════════════════════════════════════════════╗
// ║  022 — Credit Scoring Sidecar (Process Isolation)         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   FinTech — Credit Risk Assessment                ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: #[vil_sidecar], ServiceCtx, ShmSlice            ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Credit scoring for loan applications.           ║
// ║  #[vil_sidecar] isolates scoring logic in a separate       ║
// ║  process — communicates via SHM+UDS zero-copy IPC.         ║
// ║                                                             ║
// ║  NOTE: This example is pure Rust. In production, sidecar   ║
// ║  is for polyglot integration (Python ML, Go service).      ║
// ║  For pure Rust, use native functions (no sidecar overhead). ║
// ║  This example demonstrates the PATTERN for when you need   ║
// ║  process isolation or polyglot workloads.                   ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p basic-usage-sidecar-python
// Test:
//   curl http://localhost:8080/api/credit/health
//   curl -X POST http://localhost:8080/api/credit/score \
//     -H 'Content-Type: application/json' \
//     -d '{"nik":"3201234567890001","income":15000000,"loan_amount":50000000,"employment_years":3,"existing_debt":5000000}'

use vil_server::prelude::*;
use vil_server_macros::vil_sidecar;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
struct CreditScoreRequest {
    nik: String,
    income: i64,
    loan_amount: i64,
    employment_years: i32,
    existing_debt: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct CreditScoreResult {
    nik: String,
    score: f64,
    risk_class: String,
    dti_ratio: f64,
    ltv_ratio: f64,
    employment_score: f64,
    recommendation: String,
    factors: Vec<String>,
}

// ── Sidecar Functions ────────────────────────────────────────────────────
// Process-isolated credit scoring. In production, this would be a Python
// model (XGBoost/LightGBM) trained on historical loan data.
// #[vil_sidecar] handles: process spawn, SHM+UDS, invoke, timeout.

/// Comprehensive credit scoring — DTI, LTV, employment stability,
/// combined weighted score → risk classification.
#[vil_sidecar(target = "credit-scorer")]
async fn score_credit(data: &[u8]) -> CreditScoreResult {
    let req: CreditScoreRequest = serde_json::from_slice(data).unwrap_or_else(|_| {
        CreditScoreRequest {
            nik: String::new(),
            income: 0,
            loan_amount: 0,
            employment_years: 0,
            existing_debt: 0,
        }
    });

    let mut factors = Vec::new();

    // ── Debt-to-Income ratio (weight: 40%) ──
    let dti = if req.income > 0 {
        (req.existing_debt + req.loan_amount) as f64 / (req.income as f64 * 12.0)
    } else {
        1.0
    };
    let dti_score = if dti < 0.30 {
        factors.push("dti:excellent(<30%)".into());
        100.0
    } else if dti < 0.40 {
        factors.push(format!("dti:good({:.0}%)", dti * 100.0));
        80.0
    } else if dti < 0.50 {
        factors.push(format!("dti:moderate({:.0}%)", dti * 100.0));
        60.0
    } else {
        factors.push(format!("dti:high({:.0}%)", dti * 100.0));
        30.0
    };

    // ── Loan-to-Value / income coverage (weight: 30%) ──
    let ltv = if req.income > 0 {
        req.loan_amount as f64 / (req.income as f64 * 12.0 * 5.0) // 5-year capacity
    } else {
        1.0
    };
    let ltv_score = if ltv < 0.50 {
        factors.push("ltv:conservative".into());
        100.0
    } else if ltv < 0.80 {
        factors.push(format!("ltv:moderate({:.0}%)", ltv * 100.0));
        70.0
    } else {
        factors.push(format!("ltv:stretched({:.0}%)", ltv * 100.0));
        40.0
    };

    // ── Employment stability (weight: 30%) ──
    let emp_score = if req.employment_years >= 5 {
        factors.push(format!("employment:stable({}yr)", req.employment_years));
        100.0
    } else if req.employment_years >= 2 {
        factors.push(format!("employment:moderate({}yr)", req.employment_years));
        70.0
    } else {
        factors.push(format!("employment:new({}yr)", req.employment_years));
        40.0
    };

    // ── Combined weighted score ──
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

    CreditScoreResult {
        nik: req.nik,
        score: combined,
        risk_class: risk_class.into(),
        dti_ratio: dti,
        ltv_ratio: ltv,
        employment_score: emp_score,
        recommendation: recommendation.into(),
        factors,
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /score — Credit scoring via sidecar-isolated process.
async fn credit_score(
    _ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<CreditScoreResult>> {
    let input_bytes = body.as_bytes();
    let result = score_credit(input_bytes).await;
    Ok(VilResponse::ok(result))
}

/// GET /health
async fn health() -> VilResponse<serde_json::Value> {
    VilResponse::ok(serde_json::json!({
        "status": "healthy",
        "service": "credit-scorer",
        "execution_mode": "sidecar (process isolation)",
    }))
}

// ── Main — zero plumbing ─────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let credit = ServiceProcess::new("credit")
        .endpoint(Method::POST, "/score", post(credit_score))
        .endpoint(Method::GET, "/health", get(health));

    VilApp::new("sidecar-credit-scorer")
        .port(8080)
        .observer(true)
        .service(credit)
        .run()
        .await;
}
