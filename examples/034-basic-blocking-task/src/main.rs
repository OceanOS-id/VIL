// ╔════════════════════════════════════════════════════════════════════════╗
// ║  034 — Credit Risk Scoring Engine (Blocking Task)                   ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: ExecClass::BlockingTask, spawn_blocking()                 ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: A bank receives a loan application and must calculate     ║
// ║  the applicant's credit risk score. The scoring algorithm runs a     ║
// ║  CPU-intensive Monte Carlo simulation (thousands of random scenarios ║
// ║  to estimate default probability). This MUST NOT run on the async    ║
// ║  executor because it would starve other HTTP handlers.               ║
// ║                                                                      ║
// ║  Why ExecClass::BlockingTask matters:                                ║
// ║    - Async executors assume tasks yield quickly (~microseconds)      ║
// ║    - Monte Carlo simulation takes ~100ms–5s (CPU-bound, no yield)   ║
// ║    - spawn_blocking() moves the work to a dedicated thread pool      ║
// ║    - Other HTTP requests (health checks, status queries) continue    ║
// ║      to be served with sub-millisecond latency                       ║
// ║                                                                      ║
// ║  Flow:                                                               ║
// ║    1. POST /api/risk/assess → receive loan application               ║
// ║    2. spawn_blocking() → Monte Carlo simulation on blocking pool     ║
// ║    3. Return risk score + confidence interval + default probability  ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-basic-blocking-task
// Test: curl -X POST http://localhost:8080/api/risk/assess \
//         -H 'Content-Type: application/json' \
//         -d '{"applicant_id":12345,"annual_income_cents":7500000,"debt_cents":2500000,"credit_history_months":60,"simulations":500000}'
//       curl http://localhost:8080/api/risk/health

use vil_server::prelude::*;

// ── Business Domain Types ───────────────────────────────────────────────

/// Loan application submitted by a bank customer.
/// The risk engine needs income, existing debt, and credit history length
/// to run the Monte Carlo simulation.
#[derive(Deserialize)]
struct RiskAssessmentRequest {
    applicant_id: u64,
    annual_income_cents: u64,
    debt_cents: u64,
    credit_history_months: u32,
    simulations: u64,
}

/// Risk score result returned to the loan officer.
/// - score: 0.0 (no risk) to 1.0 (certain default)
/// - confidence: how tight the estimate is (higher = more reliable)
/// - default_probability: estimated chance the applicant will default
#[derive(Serialize)]
struct RiskScore {
    applicant_id: u64,
    score: f64,
    confidence: f64,
    default_probability: f64,
    simulations_run: u64,
    risk_tier: &'static str,
    exec_class: &'static str,
}

// ── Monte Carlo Simulation ──────────────────────────────────────────────

/// CPU-intensive Monte Carlo simulation for credit risk.
///
/// This function MUST NOT run on the async executor. It loops millions
/// of times without yielding, which would block all other tasks on the
/// same executor thread (health checks, other API requests, etc.).
///
/// The algorithm simulates random economic scenarios and counts how many
/// result in the applicant defaulting on the loan. The ratio of defaults
/// to total simulations gives the default probability.
fn monte_carlo_risk_simulation(
    income: u64,
    debt: u64,
    history_months: u32,
    simulations: u64,
) -> (f64, f64, f64) {
    // Debt-to-income ratio: key risk factor (higher = more risky)
    let dti = debt as f64 / income.max(1) as f64;

    // Credit history factor: longer history = lower risk
    let history_factor = 1.0 / (1.0 + history_months as f64 / 120.0);

    // Run Monte Carlo: simulate random economic scenarios
    let mut defaults = 0u64;
    let mut score_sum = 0.0f64;
    for i in 1..=simulations {
        // Pseudo-random perturbation based on iteration
        // (In production: use a proper PRNG seeded per simulation)
        let noise = ((i as f64 * 2.7182818).sin() + 1.0) / 2.0; // 0.0..1.0
        let scenario_risk = dti * 0.5 + history_factor * 0.3 + noise * 0.2;
        score_sum += scenario_risk;

        // Default threshold: if scenario risk exceeds 0.6, count as default
        if scenario_risk > 0.6 {
            defaults += 1;
        }
    }

    let avg_score = (score_sum / simulations as f64 * 1000.0).round() / 1000.0;
    let default_prob = (defaults as f64 / simulations as f64 * 10000.0).round() / 10000.0;
    // Confidence increases with more simulations (law of large numbers)
    let confidence = (1.0 - 1.0 / (simulations as f64).sqrt()) * 100.0;
    let confidence = (confidence * 100.0).round() / 100.0;

    (avg_score, confidence, default_prob)
}

// ── Risk Assessment Handler ─────────────────────────────────────────────

/// Handler for credit risk assessment.
///
/// KEY VIL FEATURE: spawn_blocking() + ExecClass::BlockingTask
/// The Monte Carlo simulation runs on tokio's blocking thread pool,
/// not on the async executor. This ensures other handlers (health checks,
/// status queries) continue to respond with sub-ms latency.
async fn assess_risk(body: ShmSlice) -> Result<VilResponse<RiskScore>, VilError> {
    let req: RiskAssessmentRequest = body.json()
        .map_err(|_| VilError::bad_request("Invalid risk assessment JSON — need applicant_id, annual_income_cents, debt_cents, credit_history_months, simulations"))?;

    let applicant_id = req.applicant_id;
    let income = req.annual_income_cents;
    let debt = req.debt_cents;
    let history = req.credit_history_months;
    let sims = req.simulations.min(10_000_000); // Cap at 10M to prevent abuse

    // Move the CPU-intensive simulation to the blocking thread pool.
    // Without spawn_blocking(), this would freeze all other HTTP handlers.
    let (score, confidence, default_prob) = tokio::task::spawn_blocking(move || {
        monte_carlo_risk_simulation(income, debt, history, sims)
    })
    .await
    .map_err(|e| VilError::internal(format!("Risk simulation failed: {}", e)))?;

    // Classify into risk tiers based on score (bank policy)
    let risk_tier = match score {
        s if s < 0.3 => "LOW_RISK — approve with standard rate",
        s if s < 0.5 => "MEDIUM_RISK — approve with higher rate",
        s if s < 0.7 => "HIGH_RISK — manual review required",
        _ => "VERY_HIGH_RISK — decline recommended",
    };

    Ok(VilResponse::ok(RiskScore {
        applicant_id,
        score,
        confidence,
        default_probability: default_prob,
        simulations_run: sims,
        risk_tier,
        exec_class: "BlockingTask — ran on spawn_blocking pool",
    }))
}

/// Health check on async executor (ExecClass::AsyncTask, default).
/// This remains fast even while Monte Carlo simulations are running.
async fn risk_health() -> VilResponse<RiskScore> {
    VilResponse::ok(RiskScore {
        applicant_id: 0,
        score: 0.0,
        confidence: 0.0,
        default_probability: 0.0,
        simulations_run: 0,
        risk_tier: "N/A — health check",
        exec_class: "AsyncTask — runs on async executor (fast, no blocking)",
    })
}

#[tokio::main]
async fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  034 — Credit Risk Scoring Engine (Blocking Task)                    ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  Monte Carlo simulation → spawn_blocking pool (not async executor)   ║");
    println!("║  Health checks remain fast while simulations run in background       ║");
    println!("║  ExecClass: AsyncTask (default) vs BlockingTask (CPU-bound)          ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let risk_svc = ServiceProcess::new("risk-engine")
        .exec(ExecClass::BlockingTask)
        .endpoint(Method::POST, "/risk/assess", post(assess_risk))
        .endpoint(Method::GET, "/risk/health", get(risk_health));

    VilApp::new("credit-risk-scoring-engine")
        .port(8080)
        .service(risk_svc)
        .run()
        .await;
}
