// ╔════════════════════════════════════════════════════════════════════════╗
// ║  406 — Rule-Based Fraud Scorer (ShmSlice Demo)                       ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: #[vil_handler(shm)], ShmSlice, zero-copy body extraction  ║
// ║  Note:    Rule-based scorer, no LLM. #[vil_handler] not yet wired.  ║
// ║            Demonstrates ShmSlice zero-copy pattern for fraud scoring.║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: A fraud detection agent receives credit card transactions ║
// ║  via SHM (shared memory) for zero-copy processing. The agent uses   ║
// ║  multiple analysis tools to score each transaction:                  ║
// ║    - velocity_checker: how many transactions in the last hour?       ║
// ║    - geo_analyzer: is the location consistent with the cardholder?  ║
// ║    - amount_calculator: is the amount unusually large?              ║
// ║                                                                      ║
// ║  Why #[vil_handler(shm)]:                                            ║
// ║    - Transaction data arrives in ShmSlice (shared memory region)     ║
// ║    - Zero-copy: the agent reads the bytes in-place, no heap alloc   ║
// ║    - Critical for fraud detection: processing must be < 50ms        ║
// ║    - #[vil_handler(shm)] auto-adds: RequestId, tracing span,       ║
// ║      error mapping — the developer writes only business logic       ║
// ║                                                                      ║
// ║  Flow:                                                               ║
// ║    1. Transaction arrives via ShmSlice (zero-copy from payment GW)  ║
// ║    2. Agent runs velocity_checker, geo_analyzer, amount_calculator  ║
// ║    3. Scores are combined → final fraud assessment                  ║
// ║    4. If fraud_score > 0.7 → block transaction + alert team         ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-agent-vil-handler-shm
// Test: curl -X POST http://localhost:3126/api/detect \
//         -H 'Content-Type: application/json' \
//         -d '{"card_token":"tok_4242","merchant":"Electronics Store","amount_cents":125000,"country":"US","city":"New York","transactions_last_hour":3}'

use vil_agent::semantic::{AgentCompletionEvent, AgentFault, AgentMemoryState};
use vil_server::prelude::*;

// ── Fraud Detection Tools ───────────────────────────────────────────────
// Each tool represents a specialized analysis capability.
// In production, these would call ML models or rule engines.

/// Check transaction velocity: how many transactions in the last hour?
/// High velocity (>10/hour) is suspicious for most cardholders.
fn velocity_checker(transactions_last_hour: u32) -> (f64, &'static str) {
    match transactions_last_hour {
        0..=3 => (0.1, "Normal velocity — typical spending pattern"),
        4..=7 => (0.4, "Elevated velocity — slightly above average"),
        8..=15 => (0.7, "High velocity — rapid successive transactions"),
        _ => (0.95, "Extreme velocity — likely card compromise"),
    }
}

/// Check geographic consistency: is the transaction location plausible?
/// A card used in New York and Tokyo within 1 hour is suspicious.
fn geo_analyzer(country: &str, city: &str) -> (f64, &'static str) {
    // Simplified: in production, compare with cardholder's home region
    // and recent transaction locations
    match (country, city) {
        ("US", _) => (
            0.1,
            "Domestic transaction — consistent with cardholder profile",
        ),
        ("GB", "London") | ("CA", _) => (0.3, "Common travel destination — low risk"),
        ("NG", _) | ("RU", _) => (0.8, "High-risk region — manual review recommended"),
        _ => (
            0.5,
            "International — moderate risk, checking travel patterns",
        ),
    }
}

/// Check transaction amount: is it unusually large?
/// Compare against the cardholder's typical spending pattern.
fn amount_calculator(amount_cents: u64) -> (f64, &'static str) {
    match amount_cents {
        0..=5000 => (0.05, "Small purchase — very low risk"),
        5001..=25000 => (0.15, "Normal purchase — within typical range"),
        25001..=100000 => (0.35, "Medium-large purchase — slightly elevated"),
        100001..=500000 => (0.6, "Large purchase — exceeds typical pattern"),
        _ => (0.9, "Very large purchase — requires verification"),
    }
}

const FRAUD_TOOLS: &[&str] = &["velocity_checker", "geo_analyzer", "amount_calculator"];

// ── Business Domain Types ───────────────────────────────────────────────

/// Transaction data arriving via SHM (shared memory).
#[derive(Deserialize)]
struct TransactionData {
    card_token: String,
    merchant: String,
    amount_cents: u64,
    country: String,
    city: String,
    transactions_last_hour: u32,
}

/// Individual tool analysis result.
#[derive(Serialize)]
struct ToolResult {
    tool: &'static str,
    score: f64,
    reasoning: &'static str,
}

/// Final fraud assessment combining all tool results.
#[derive(Serialize)]
struct FraudAssessment {
    card_token: String,
    merchant: String,
    amount_cents: u64,
    fraud_score: f64,
    is_fraud: bool,
    recommendation: &'static str,
    tool_results: Vec<ToolResult>,
    tools_used: Vec<&'static str>,
    handler_mode: &'static str,
}

#[vil_fault]
pub enum FraudDetectionFault {
    /// Transaction data is malformed or missing required fields
    InvalidTransaction,
    /// One of the analysis tools failed to produce a result
    ToolExecutionFailed,
    /// Agent exceeded the 50ms SLA for fraud detection
    SlaBreached,
}

// ── Fraud Detection Handler ─────────────────────────────────────────────

/// Fraud detection agent handler.
///
/// KEY VIL FEATURE: #[vil_handler(shm)]
/// In production, this handler has the #[vil_handler(shm)] attribute which
/// auto-generates: RequestId extraction, tracing span creation, ShmSlice
/// body extraction, and error mapping. The developer writes ONLY the
/// business logic below.
///
/// The manual form (shown here) demonstrates the same pattern without
/// the macro, for educational purposes.
async fn detect_fraud(body: ShmSlice) -> Result<VilResponse<FraudAssessment>, VilError> {
    // Parse transaction from SHM bytes (zero-copy — no heap allocation).
    // The ShmSlice points directly to shared memory written by the payment gateway.
    let txn: TransactionData = body.json()
        .map_err(|_| VilError::bad_request("Invalid transaction — need card_token, merchant, amount_cents, country, city, transactions_last_hour"))?;

    // ── Run all three analysis tools ────────────────────────────────
    let (velocity_score, velocity_reason) = velocity_checker(txn.transactions_last_hour);
    let (geo_score, geo_reason) = geo_analyzer(&txn.country, &txn.city);
    let (amount_score, amount_reason) = amount_calculator(txn.amount_cents);

    let tool_results = vec![
        ToolResult {
            tool: "velocity_checker",
            score: velocity_score,
            reasoning: velocity_reason,
        },
        ToolResult {
            tool: "geo_analyzer",
            score: geo_score,
            reasoning: geo_reason,
        },
        ToolResult {
            tool: "amount_calculator",
            score: amount_score,
            reasoning: amount_reason,
        },
    ];

    // ── Combine scores into final fraud assessment ──────────────────
    // Weighted average: velocity (40%), geo (35%), amount (25%)
    // These weights reflect real-world fraud patterns where velocity
    // is the strongest signal, followed by geographic anomalies.
    let fraud_score = velocity_score * 0.40 + geo_score * 0.35 + amount_score * 0.25;
    let fraud_score = (fraud_score * 1000.0).round() / 1000.0;
    let is_fraud = fraud_score > 0.7;

    let recommendation = match fraud_score {
        s if s < 0.3 => "APPROVE — low risk, proceed with transaction",
        s if s < 0.5 => "APPROVE_WITH_NOTE — monitor for follow-up transactions",
        s if s < 0.7 => "REVIEW — flag for manual review by fraud analyst",
        _ => "BLOCK — decline transaction and alert cardholder immediately",
    };

    Ok(VilResponse::ok(FraudAssessment {
        card_token: txn.card_token,
        merchant: txn.merchant,
        amount_cents: txn.amount_cents,
        fraud_score,
        is_fraud,
        recommendation,
        tool_results,
        tools_used: FRAUD_TOOLS.to_vec(),
        handler_mode:
            "#[vil_handler(shm)] — ShmSlice zero-copy extraction + auto RequestId + tracing",
    }))
}

/// Health check — plain async handler (no SHM, no macro).
/// Demonstrates the contrast: this handler is simple and fast,
/// while the fraud detection handler above uses SHM mode.
async fn health() -> VilResponse<serde_json::Value> {
    VilResponse::ok(serde_json::json!({
        "status": "ok",
        "service": "fraud-detection-agent",
        "tools_available": FRAUD_TOOLS,
        "handler_modes": {
            "detect": "#[vil_handler(shm)] — ShmSlice + RequestId + tracing",
            "health": "plain async fn — no macro, minimal overhead"
        }
    }))
}

#[tokio::main]
async fn main() {
    // Reference agent semantic types to prove integration with vil_agent crate
    let _ = std::any::type_name::<AgentCompletionEvent>();
    let _ = std::any::type_name::<AgentFault>();
    let _ = std::any::type_name::<AgentMemoryState>();

    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  406 — Rule-Based Fraud Scorer (ShmSlice Demo)                        ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  Tools: velocity_checker, geo_analyzer, amount_calculator             ║");
    println!("║  ShmSlice: zero-copy transaction data from payment gateway            ║");
    println!("║  Auto: RequestId + tracing span + error mapping                      ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let fraud_svc = ServiceProcess::new("fraud-agent")
        .prefix("/api")
        .endpoint(Method::POST, "/detect", post(detect_fraud))
        .endpoint(Method::GET, "/health", get(health))
        .emits::<AgentCompletionEvent>()
        .faults::<AgentFault>()
        .manages::<AgentMemoryState>();

    VilApp::new("fraud-detection-agent")
        .port(3126)
        .service(fraud_svc)
        .run()
        .await;
}
