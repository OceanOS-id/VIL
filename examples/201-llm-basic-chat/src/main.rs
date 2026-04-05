// ╔════════════════════════════════════════════════════════════╗
// ║  201 — Medical Triage Chatbot                             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Healthcare / Patient Pre-Screening             ║
// ║  Pattern:  VX_APP                                         ║
// ║  Features: ShmSlice, ServiceCtx, VilResponse, SseCollect, ║
// ║            LlmResponseEvent, LlmFault, LlmUsageState     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: AI-assisted patient symptom assessment.         ║
// ║  Every interaction produces LlmResponseEvent (Data Lane)  ║
// ║  for clinical audit. LlmUsageState tracks cumulative      ║
// ║  token usage for budget compliance. LlmFault alerts ops.  ║
// ╚════════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator at localhost:4545
//
// Run:   cargo run -p llm-plugin-usage-basic-chat
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "I have chest pain and shortness of breath"}' \
//     http://localhost:3100/api/chat
//   curl http://localhost:3100/api/usage

use std::sync::{Arc, Mutex};
use std::time::Instant;

use vil_llm::semantic::{LlmFault, LlmResponseEvent, LlmUsageState};
use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Fault: typed error conditions for medical triage ─────────────────
// Each fault triggers different alerting — EmptyResponse in triage is critical.

#[vil_fault]
pub enum ChatFault {
    UpstreamTimeout,
    EmptyResponse,
    InvalidPrompt,
}

// ── Request / Response ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ChatRequest {
    prompt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ChatResponse {
    content: String,
    model: String,
    prompt_tokens_approx: u32,
    completion_tokens_approx: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct UsageResponse {
    provider: String,
    total_requests: u64,
    total_tokens: u64,
    total_errors: u64,
    avg_latency_ms: f64,
}

// ── Shared state: LlmUsageState from vil_llm::semantic ──────────────

struct AppState {
    usage: Mutex<LlmUsageState>,
}

// ── Handler: triage chat ─────────────────────────────────────────────

async fn chat_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<ChatResponse>> {
    let req: ChatRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    if req.prompt.trim().is_empty() {
        return Err(VilError::bad_request("prompt is required"));
    }

    let start = Instant::now();

    let body_json = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {
                "role": "system",
                "content": "You are a medical triage assistant. Assess symptoms, classify urgency \
                           (EMERGENCY/URGENT/ROUTINE), and recommend next steps. Be concise and clear."
            },
            { "role": "user", "content": req.prompt }
        ],
        "stream": true
    });

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .dialect(SseDialect::openai())
        .body(body_json);

    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    let content = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    let latency_ms = start.elapsed().as_millis() as u64;
    let prompt_tokens = req.prompt.split_whitespace().count() as u32;
    let completion_tokens = content.split_whitespace().count() as u32;

    // ── Construct LlmResponseEvent (from vil_llm::semantic) ──
    // This is the semantic audit record — flows on Data Lane.
    // NOT dead code — used to update LlmUsageState.
    let event = LlmResponseEvent {
        provider: "openai".into(),
        model: "gpt-4".into(),
        prompt_tokens,
        completion_tokens,
        total_tokens: prompt_tokens + completion_tokens,
        latency_ms,
        finish_reason: "stop".into(),
        cached: false,
    };

    // ── Update LlmUsageState (from vil_llm::semantic) ──
    // Cumulative tracking: total requests, tokens, avg latency.
    // In healthcare: feeds budget compliance and quality dashboards.
    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;
    state.usage.lock().unwrap().record(&event);

    Ok(VilResponse::ok(ChatResponse {
        content,
        model: event.model,
        prompt_tokens_approx: prompt_tokens,
        completion_tokens_approx: completion_tokens,
    }))
}

// ── Handler: usage stats ─────────────────────────────────────────────
// Exposes LlmUsageState for monitoring dashboards.

async fn usage_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<UsageResponse>> {
    let state = ctx.state::<Arc<AppState>>()
        .map_err(|_| VilError::internal("state not found"))?;
    let usage = state.usage.lock().unwrap();

    Ok(VilResponse::ok(UsageResponse {
        provider: usage.provider.clone(),
        total_requests: usage.total_requests,
        total_tokens: usage.total_tokens,
        total_errors: usage.total_errors,
        avg_latency_ms: usage.avg_latency_ms,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  201 — Medical Triage Chatbot (VilApp + LLM Semantic)       ║");
    println!("║  Events: LlmResponseEvent | Faults: LlmFault               ║");
    println!("║  State: LlmUsageState (cumulative token tracking)           ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Chat:  http://localhost:3100/api/chat");
    println!("  Usage: http://localhost:3100/api/usage");
    println!("  Upstream: {}", UPSTREAM_URL);
    println!();

    let app_state = Arc::new(AppState {
        usage: Mutex::new(LlmUsageState {
            provider: "openai".into(),
            ..Default::default()
        }),
    });

    let svc = ServiceProcess::new("chat")
        .prefix("/api")
        .emits::<LlmResponseEvent>()
        .faults::<LlmFault>()
        .manages::<LlmUsageState>()
        .endpoint(Method::POST, "/chat", post(chat_handler))
        .endpoint(Method::GET, "/usage", get(usage_handler))
        .state(app_state);

    VilApp::new("llm-basic-chat")
        .port(3100)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
