// ╔════════════════════════════════════════════════════════════╗
// ║  206 — Insurance Underwriting AI (LLM Risk Assessment)    ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Insurance — Underwriting Decision Support       ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: LlmProvider, ChatMessage, ServiceCtx, ShmSlice ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Underwriter submits applicant profile → LLM     ║
// ║  assesses risk factors, generates narrative, recommends    ║
// ║  premium tier. Rule-based pre-screen + LLM assessment.     ║
// ╚════════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator at localhost:4545
//
// Run:   cargo run -p vil-llm-decision-routing
// Test:
//   curl -X POST http://localhost:8080/api/underwrite/assess \
//     -H 'Content-Type: application/json' \
//     -d '{"applicant_age":35,"occupation":"software_engineer","health_score":"good","coverage_cents":200000000}'

use std::sync::Arc;

use vil_llm::{ChatMessage, LlmProvider, OpenAiConfig, OpenAiProvider};
use vil_server::prelude::*;

#[derive(Debug, Deserialize)]
struct UnderwriteRequest {
    applicant_age: u32,
    occupation: String,
    health_score: String,
    coverage_cents: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct UnderwriteResponse {
    applicant_age: u32,
    pre_screen: PreScreen,
    ai_assessment: String,
    recommended_tier: String,
    premium_estimate_cents: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct PreScreen {
    passed: bool,
    reason: String,
}

struct UnderwriteState {
    llm: Arc<dyn LlmProvider>,
}

/// POST /assess — Underwriting risk assessment with LLM.
async fn assess(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<UnderwriteResponse>> {
    let req: UnderwriteRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    // ── Rule-based pre-screen (native, instant) ──
    if req.applicant_age < 18 {
        return Ok(VilResponse::ok(UnderwriteResponse {
            applicant_age: req.applicant_age,
            pre_screen: PreScreen { passed: false, reason: "under 18".into() },
            ai_assessment: "N/A — pre-screen rejected".into(),
            recommended_tier: "DECLINE".into(),
            premium_estimate_cents: 0,
        }));
    }
    if req.coverage_cents > 500_000_000 {
        return Ok(VilResponse::ok(UnderwriteResponse {
            applicant_age: req.applicant_age,
            pre_screen: PreScreen { passed: false, reason: "coverage > $5M requires manual review".into() },
            ai_assessment: "N/A — pre-screen: manual review required".into(),
            recommended_tier: "MANUAL_REVIEW".into(),
            premium_estimate_cents: 0,
        }));
    }

    // ── LLM risk assessment (actual AI call) ──
    let state = ctx.state::<Arc<UnderwriteState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let messages = vec![
        ChatMessage::system(
            "You are an insurance underwriter AI. Analyze the applicant and respond with EXACTLY this format:\n\
             TIER: [Preferred|Standard|Substandard|Decline]\n\
             ASSESSMENT: [2-3 sentence risk analysis]"
        ),
        ChatMessage::user(&format!(
            "Applicant: age={}, occupation={}, health={}, coverage=${}",
            req.applicant_age,
            req.occupation,
            req.health_score,
            req.coverage_cents / 100
        )),
    ];

    let response = state.llm.chat(&messages).await
        .map_err(|e| VilError::internal(format!("LLM failed: {}", e)))?;

    // Parse tier from LLM response
    let content = &response.content;
    let tier = if content.contains("Preferred") {
        "Preferred"
    } else if content.contains("Substandard") {
        "Substandard"
    } else if content.contains("Decline") {
        "Decline"
    } else {
        "Standard"
    };

    // Premium estimate based on tier + coverage
    let rate_bps = match tier {
        "Preferred" => 50,      // 0.5%
        "Standard" => 100,      // 1.0%
        "Substandard" => 200,   // 2.0%
        _ => 0,
    };
    let premium = req.coverage_cents * rate_bps / 10000;

    Ok(VilResponse::ok(UnderwriteResponse {
        applicant_age: req.applicant_age,
        pre_screen: PreScreen { passed: true, reason: "passed".into() },
        ai_assessment: response.content,
        recommended_tier: tier.into(),
        premium_estimate_cents: premium,
    }))
}

#[tokio::main]
async fn main() {
    let upstream = std::env::var("LLM_UPSTREAM")
        .unwrap_or_else(|_| "http://127.0.0.1:4545".into());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let provider = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-4").base_url(&format!("{}/v1", upstream)),
    ));

    let state = Arc::new(UnderwriteState { llm: provider });

    let svc = ServiceProcess::new("underwrite")
        .endpoint(Method::POST, "/assess", post(assess))
        .state(state);

    VilApp::new("insurance-underwriting-ai")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
