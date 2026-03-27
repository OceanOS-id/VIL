// ╔════════════════════════════════════════════════════════════════════════╗
// ║  206 — Insurance Underwriting AI (LLM Decision Routing)             ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: #[vil_decision] POD-only semantic type,                   ║
// ║            Control Lane routing decision                             ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: An insurance company uses AI to underwrite new policies.  ║
// ║  The AI analyzes risk factors (age, health history, occupation) and  ║
// ║  decides which premium tier to assign:                               ║
// ║    - Preferred: low risk → lowest premiums                           ║
// ║    - Standard: average risk → normal premiums                        ║
// ║    - High-Risk: elevated risk → higher premiums + medical review     ║
// ║                                                                      ║
// ║  Why #[vil_decision]:                                                ║
// ║    - The decision struct travels on the Control Lane (not Data Lane) ║
// ║    - Control Lane is never blocked by bulk data (medical records)   ║
// ║    - POD-only fields (no String/Vec) → the decision fits in a       ║
// ║      single cache line (64 bytes), enabling zero-copy on Control    ║
// ║    - Downstream services read the decision to route the policy      ║
// ║      to the correct pricing engine without deserializing the full   ║
// ║      application                                                    ║
// ║                                                                      ║
// ║  Flow:                                                               ║
// ║    1. Agent submits underwriting request (applicant data)            ║
// ║    2. AI scores risk factors → produces PremiumDecision              ║
// ║    3. PremiumDecision rides Control Lane → pricing engine            ║
// ║    4. Full application rides Data Lane → document storage            ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-llm-decision-routing
// Test: curl -X POST http://localhost:3116/api/underwrite \
//         -H 'Content-Type: application/json' \
//         -d '{"applicant_name":"Jane Smith","age":35,"occupation":"software_engineer","health_score":"excellent","coverage_cents":50000000}'

use vil_server::prelude::*;
use vil_llm::semantic::{LlmResponseEvent, LlmFault, LlmUsageState};

// ── Premium Tier Constants ──────────────────────────────────────────────
// These are integer codes because #[vil_decision] requires POD-only fields.
// No String, no Vec — just Copy types that fit in a CPU cache line.
const TIER_PREFERRED: u8 = 1;   // Low risk — best rates
const TIER_STANDARD: u8 = 2;    // Average risk — normal rates
const TIER_HIGH_RISK: u8 = 3;   // Elevated risk — surcharge + review

// ── AI Decision (POD-only for Control Lane) ─────────────────────────────
//
// #[vil_decision] enforces that ALL fields are Copy (no heap allocation).
// This is critical because the decision travels on the Control Lane,
// which uses fixed-size slots in SHM for predictable, zero-copy routing.
// A decision must NEVER block the Control Lane with variable-size data.
#[vil_decision]
pub struct PremiumDecision {
    /// Which premium tier: TIER_PREFERRED (1), TIER_STANDARD (2), TIER_HIGH_RISK (3)
    pub tier: u8,
    /// AI confidence in the decision: 0-100 (higher = more certain)
    pub confidence: u64,
    /// Numeric risk score: 0 (no risk) to 1000 (maximum risk)
    pub risk_score: u64,
    /// Whether this decision came from cache (previous identical applicant)
    pub is_cached: bool,
}

// ── Reason Codes ────────────────────────────────────────────────────────
const REASON_AGE_HEALTH: u8 = 1;   // Decision driven by age + health factors
const REASON_OCCUPATION: u8 = 2;   // Decision driven by occupation risk
const REASON_COVERAGE: u8 = 3;     // Decision driven by coverage amount

// ── Business Fault Types ────────────────────────────────────────────────
#[vil_fault]
pub enum UnderwritingFault {
    /// No AI model available to score the application
    ModelUnavailable,
    /// AI scoring took too long (SLA breach)
    ScoringTimeout,
    /// Input data is invalid or incomplete
    InvalidApplication,
}

// ── Business Domain Types ───────────────────────────────────────────────

/// Underwriting request from an insurance agent.
#[derive(Deserialize)]
struct UnderwritingRequest {
    applicant_name: String,
    age: u32,
    occupation: String,
    health_score: String,
    coverage_cents: u64,
}

/// Underwriting response with the AI's premium decision.
#[derive(Serialize)]
struct UnderwritingResponse {
    applicant_name: String,
    tier_name: &'static str,
    tier_code: u8,
    confidence_percent: u64,
    risk_score: u64,
    reason: &'static str,
    monthly_premium_cents: u64,
    requires_medical_review: bool,
}

// ── Underwriting Handler ────────────────────────────────────────────────

/// AI underwriting endpoint.
///
/// KEY VIL FEATURE: #[vil_decision]
/// The PremiumDecision struct is POD-only (no heap types). It rides the
/// Control Lane to downstream pricing engines. Meanwhile, the full
/// application (with medical records, documents) rides the Data Lane.
/// The pricing engine reads ONLY the 32-byte decision to route the policy
/// — it never needs to deserialize the multi-KB application payload.
async fn underwrite(body: ShmSlice) -> Result<VilResponse<UnderwritingResponse>, VilError> {
    let req: UnderwritingRequest = body.json()
        .map_err(|_| VilError::bad_request("Invalid application — need applicant_name, age, occupation, health_score, coverage_cents"))?;

    // ── AI Risk Scoring Logic ───────────────────────────────────────
    // In production: call an ML model trained on historical claims data.
    // Here we use a rule-based approximation for demonstration.

    // Age factor: younger applicants generally lower risk
    let age_risk = match req.age {
        18..=30 => 100,
        31..=45 => 200,
        46..=60 => 400,
        _ => 600,
    };

    // Health factor: self-reported health score
    let health_risk = match req.health_score.as_str() {
        "excellent" => 50,
        "good" => 150,
        "fair" => 350,
        "poor" => 600,
        _ => 300,
    };

    // Occupation factor: some jobs carry higher risk
    let occupation_risk = match req.occupation.as_str() {
        "software_engineer" | "teacher" | "accountant" => 50,
        "construction_worker" | "firefighter" => 300,
        "pilot" | "deep_sea_diver" => 500,
        _ => 150,
    };

    // Combined risk score (0-1000 scale)
    let risk_score = ((age_risk + health_risk + occupation_risk) as u64).min(1000);

    // Make the AI decision based on risk score thresholds
    let decision = if risk_score < 250 {
        PremiumDecision { tier: TIER_PREFERRED, confidence: 92, risk_score, is_cached: false }
    } else if risk_score < 500 {
        PremiumDecision { tier: TIER_STANDARD, confidence: 85, risk_score, is_cached: false }
    } else {
        PremiumDecision { tier: TIER_HIGH_RISK, confidence: 78, risk_score, is_cached: false }
    };

    // Map decision to human-readable output
    let (tier_name, reason, requires_review) = match decision.tier {
        TIER_PREFERRED => ("Preferred", "Low risk profile — eligible for best rates", false),
        TIER_STANDARD => ("Standard", "Average risk — standard pricing applies", false),
        TIER_HIGH_RISK => ("High-Risk", "Elevated risk — surcharge applied, medical review required", true),
        _ => ("Unknown", "Fallback", false),
    };

    // Calculate monthly premium based on tier and coverage amount
    // (simplified: in production, actuarial tables + regional factors)
    let base_rate = match decision.tier {
        TIER_PREFERRED => 15,   // 0.015% of coverage per month
        TIER_STANDARD => 25,    // 0.025%
        TIER_HIGH_RISK => 45,   // 0.045%
        _ => 30,
    };
    let monthly_premium_cents = req.coverage_cents * base_rate / 100_000;

    Ok(VilResponse::ok(UnderwritingResponse {
        applicant_name: req.applicant_name,
        tier_name,
        tier_code: decision.tier,
        confidence_percent: decision.confidence,
        risk_score: decision.risk_score,
        reason,
        monthly_premium_cents,
        requires_medical_review: requires_review,
    }))
}

#[tokio::main]
async fn main() {
    // Reference LLM semantic types to prove integration with vil_llm crate
    let _ = std::any::type_name::<LlmResponseEvent>();
    let _ = std::any::type_name::<LlmFault>();
    let _ = std::any::type_name::<LlmUsageState>();

    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  206 — Insurance Underwriting AI (LLM Decision Routing)              ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  #[vil_decision] → POD-only, rides Control Lane (never blocked)      ║");
    println!("║  Tiers: Preferred (low risk) / Standard / High-Risk (review needed)  ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let underwriting_svc = ServiceProcess::new("underwriter")
        .prefix("/api")
        .endpoint(Method::POST, "/underwrite", post(underwrite));

    VilApp::new("insurance-underwriting-ai")
        .port(3116)
        .service(underwriting_svc)
        .run()
        .await;
}
