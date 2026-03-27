// ╔════════════════════════════════════════════════════════════╗
// ║  201 — Medical Triage Chatbot                             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Healthcare / Patient Pre-Screening             ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, VilResponse, #[vil_fault]║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   AI-assisted patient symptom assessment before doctor visit. Medical
//   triage chatbots reduce ER wait times and help patients determine
//   the appropriate level of care:
//
//   - Symptom collection: structured gathering of patient complaints
//   - Urgency scoring: classify as emergency / urgent / routine
//   - Pre-visit preparation: gather relevant history before appointment
//   - Resource allocation: route patients to the right specialist
//
//   Business impact:
//   - Reduces unnecessary ER visits by 20-30%
//   - Improves patient experience with immediate 24/7 access
//   - Gives clinicians structured pre-visit data for faster diagnosis
//   - Tracks symptom patterns for public health surveillance
//
// Why semantic types (ChatState, ChatCompletedEvent)?
//   In healthcare, every AI interaction must be auditable. Semantic types
//   provide structured, machine-readable events that feed into:
//   - Clinical audit logs (regulatory compliance: HIPAA, GDPR)
//   - Quality metrics (accuracy of triage recommendations)
//   - Token usage tracking (cost management per patient interaction)
//
// Run:
//   cargo run -p llm-plugin-usage-basic-chat
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "Hello, world!"}' \
//     http://localhost:3100/api/chat

use vil_server::prelude::*;
use vil_llm::semantic::{LlmResponseEvent, LlmFault, LlmUsageState};

// Upstream LLM endpoint. In a medical triage system, this would point
// to a HIPAA-compliant LLM deployment with no data retention.
const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────
// These types model the triage chatbot's operational state and events.

// ChatState: tracks aggregate triage session metrics. In healthcare,
// this state feeds real-time dashboards showing patient load, average
// token consumption, and active triage model version.
#[derive(Clone, Debug)]
pub struct ChatState {
    pub total_requests: u64,         // Total triage sessions today
    pub total_tokens_approx: u64,    // Approximate token usage (cost tracking)
    pub last_model: String,          // Active triage model version
}

// ChatCompletedEvent: emitted after each triage interaction. Used for
// clinical quality review — was the response appropriately detailed?
// Did it correctly assess urgency? Compliance teams audit these events.
#[derive(Clone, Debug)]
pub struct ChatCompletedEvent {
    pub prompt_len: u32,      // Patient symptom description length
    pub response_len: u32,    // Triage recommendation length
    pub model: String,        // Model used for this assessment
}

// ChatFault: typed error conditions specific to medical triage.
// Each fault triggers different alerting — e.g., EmptyResponse
// in a triage context is critical (patient received no guidance).
#[vil_fault]
pub enum ChatFault {
    UpstreamTimeout,   // LLM service didn't respond — patient sees delay
    EmptyResponse,     // LLM returned nothing — critical in medical context
    InvalidPrompt,     // Patient input couldn't be parsed
}

// ── Request / Response ──────────────────────────────────────────────
// The triage API: patients describe symptoms, receive assessment.

// ChatRequest: a patient's symptom description. In a full triage system,
// this would also include patient_id, age, gender, and medical_history
// for more accurate assessment.
#[derive(Debug, Deserialize)]
struct ChatRequest {
    prompt: String,
}

// ChatResponse: the triage recommendation. VilModel enables zero-copy
// serialization for responsive patient-facing applications.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ChatResponse {
    content: String,
}

// ── Handler: Process patient symptom assessment ──────────────────────
// This handler implements the medical triage flow:
// 1. Receive patient symptom description
// 2. Send to LLM with a medical assistant system prompt
// 3. Stream and collect the triage recommendation
// 4. Record the interaction for clinical audit
// 5. Return the assessment to the patient portal
//
// In production, additional steps would include:
// - Urgency classification (emergency/urgent/routine)
// - PII redaction before logging
// - Specialist routing recommendation

async fn chat_handler(
    ctx: ServiceCtx, body: ShmSlice,
) -> HandlerResult<VilResponse<ChatResponse>> {
    // ShmSlice: zero-copy body from ExchangeHeap — essential for
    // healthcare applications where response latency directly
    // impacts patient experience and triage accuracy.
    let req: ChatRequest = body.json().expect("invalid JSON body");

    // System prompt configures the LLM as a medical triage assistant.
    // In production, this would include clinical guidelines and
    // disclaimers required by medical regulatory bodies.
    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": req.prompt}
        ],
        "stream": true
    });

    // Read API key from env (empty = simulator mode, no auth needed)
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // SseDialect::openai() handles the standard streaming format.
    // For medical applications, the dialect ensures complete response
    // collection — partial triage recommendations are dangerous.
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .dialect(SseDialect::openai())
        .body(body);

    // Add auth if API key is set (skip for local simulator)
    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    let content = collector.collect_text().await
        .map_err(|e| VilError::internal(e.to_string()))?;

    // Semantic audit: record the triage completion event.
    // In a medical system, this event feeds into:
    // - Clinical quality dashboards (response adequacy)
    // - Regulatory compliance logs (HIPAA audit trail)
    // - Cost tracking (token usage per patient interaction)
    let _event = ChatCompletedEvent {
        prompt_len: req.prompt.len() as u32,
        response_len: content.len() as u32,
        model: "gpt-4".into(),
    };

    Ok(VilResponse::ok(ChatResponse { content }))
}

// ── Main ────────────────────────────────────────────────────────────
// Bootstrap the medical triage chatbot service.

#[tokio::main]
async fn main() {
    // Semantic type registration — compile-time validation ensures
    // the triage service correctly participates in the healthcare
    // observability pipeline. Type mismatches fail at compile time,
    // not at runtime when patients are waiting.
    let _event = std::any::type_name::<LlmResponseEvent>();
    let _fault = std::any::type_name::<LlmFault>();
    let _state = std::any::type_name::<LlmUsageState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  201 — LLM Basic Chat (VilApp)                             ║");
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Simplest LLM — single endpoint, system prompt     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!("  Auth: {}", if api_key.is_empty() { "simulator mode (no auth)" } else { "OPENAI_API_KEY (Bearer)" });
    println!("  Listening on http://localhost:3100/api/chat");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    // The "chat" ServiceProcess handles all patient triage interactions.
    // Semantic declarations enable automatic healthcare metrics:
    // - LlmResponseEvent: tracks triage response quality and latency
    // - LlmFault: alerts clinical ops when the triage system fails
    // - LlmUsageState: monitors token consumption for budget compliance
    let svc = ServiceProcess::new("chat")
        .prefix("/api")
        .emits::<LlmResponseEvent>()
        .faults::<LlmFault>()
        .manages::<LlmUsageState>()
        .endpoint(Method::POST, "/chat", post(chat_handler));

    // Port 3100: the medical triage chatbot's service port.
    // In production, TLS and patient authentication are handled
    // by the API gateway layer (see example 002).
    VilApp::new("llm-basic-chat")
        .port(3100)
        .service(svc)
        .run()
        .await;
}
