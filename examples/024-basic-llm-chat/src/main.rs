// ╔════════════════════════════════════════════════════════════╗
// ║  024 — Customer Support Chatbot                           ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Customer Service / Ticket Deflection           ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Features: ShmSlice, VilResponse, SseCollect, SseDialect, ║
// ║            LlmResponseEvent, LlmFault, LlmUsageState     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: AI-assisted customer ticket deflection.         ║
// ║  Every interaction produces LlmResponseEvent (Data Lane)  ║
// ║  for support quality audit. LlmUsageState tracks          ║
// ║  cumulative token usage for budget compliance.             ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   Handle customer queries via LLM — a ticket deflection system that
//   resolves common support questions without human agent involvement.
//   In enterprise customer service:
//
//   - 60-70% of support tickets are repetitive (password resets, billing
//     questions, shipping status) — an LLM can handle these instantly
//   - Each deflected ticket saves $5-15 in human agent cost
//   - Response time drops from hours (human queue) to seconds (LLM)
//   - Escalation to human agents happens only for complex issues
//
// Architecture:
//   Customer Portal -> [This Chatbot :3090] -> [LLM Service :4545]
//                                           -> [Ticket System] (on escalation)
//
// Run:
//   cargo run -p basic-usage-llm-chat
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is Rust?"}' \
//     http://localhost:3090/api/chat
//   curl http://localhost:3090/api/usage

use std::sync::{Arc, Mutex};
use std::time::Instant;

use vil_llm::semantic::{LlmFault, LlmResponseEvent, LlmUsageState};
use vil_server::prelude::*;

// Upstream LLM endpoint — the chatbot's "brain". In production, this
// would point to a managed LLM service or a self-hosted model
// fine-tuned on the company's support knowledge base.
const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Request / Response ──────────────────────────────────────────────
// The chatbot API: customers send their question, get an AI-generated answer.

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

// ── Handler: Process customer query through LLM ──────────────────────

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

    // The system prompt sets the chatbot's persona.
    let body_json = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": req.prompt}
        ],
        "stream": true
    });

    // Read API key from env (empty = simulator mode, no auth needed)
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // SseCollect with OpenAI dialect: the standard pattern for
    // consuming streaming LLM responses.
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .dialect(SseDialect::openai())
        .body(body_json);

    // Add auth if API key is set (skip for local simulator)
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
    // Semantic audit record — flows on Data Lane.
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

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  024 — LLM Chat (VilApp + SseCollect + LLM Semantic)       ║");
    println!("║  Events: LlmResponseEvent | Faults: LlmFault               ║");
    println!("║  State: LlmUsageState (cumulative token tracking)           ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode (no auth)"
        } else {
            "OPENAI_API_KEY (Bearer)"
        }
    );
    println!("  Chat:  http://localhost:3090/api/chat");
    println!("  Usage: http://localhost:3090/api/usage");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    let app_state = Arc::new(AppState {
        usage: Mutex::new(LlmUsageState {
            provider: "openai".into(),
            ..Default::default()
        }),
    });

    // The "chat" ServiceProcess handles all customer support interactions.
    let svc = ServiceProcess::new("chat")
        .prefix("/api")
        .emits::<LlmResponseEvent>()
        .faults::<LlmFault>()
        .manages::<LlmUsageState>()
        .endpoint(Method::POST, "/chat", post(chat_handler))
        .endpoint(Method::GET, "/usage", get(usage_handler))
        .state(app_state);

    // Port 3090: the customer support chatbot's internal service port.
    VilApp::new("llm-chat")
        .port(3090)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
