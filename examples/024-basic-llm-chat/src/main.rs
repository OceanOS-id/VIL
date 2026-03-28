// ╔════════════════════════════════════════════════════════════╗
// ║  024 — Customer Support Chatbot                           ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Customer Service / Ticket Deflection           ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Features: ShmSlice, VilResponse, SseCollect, SseDialect  ║
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
// Why SseDialect::openai()?
//   The OpenAI dialect handles the standard streaming format used by
//   most LLM providers. It automatically parses `data: [DONE]` terminators
//   and extracts `choices[0].delta.content` from each SSE chunk. This is
//   the recommended approach when working with OpenAI-compatible APIs.
//
// Run:
//   cargo run -p basic-usage-llm-chat
//
// Test:
//   curl -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is Rust?"}' \
//     http://localhost:3090/api/chat

use vil_llm::semantic::{LlmFault, LlmResponseEvent, LlmUsageState};
use vil_server::prelude::*;

// Upstream LLM endpoint — the chatbot's "brain". In production, this
// would point to a managed LLM service or a self-hosted model
// fine-tuned on the company's support knowledge base.
const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Request / Response ──────────────────────────────────────────────
// The chatbot API: customers send their question, get an AI-generated answer.

// ChatRequest represents a customer's support query. In a full system,
// this would also include session_id (for conversation history),
// customer_tier (for priority routing), and channel (web/mobile/email).
#[derive(Debug, Deserialize)]
struct ChatRequest {
    prompt: String,
}

// ChatResponse carries the AI-generated answer back to the customer.
// VilModel enables zero-copy serialization for high-throughput
// customer service portals handling thousands of concurrent sessions.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ChatResponse {
    content: String,
}

// ── Handler: Process customer query through LLM ──────────────────────
// This is the core ticket deflection logic:
// 1. Receive the customer's question
// 2. Send to LLM with a support-oriented system prompt
// 3. Stream the response via SSE and collect the full answer
// 4. Return the answer to the customer portal
//
// In production, this handler would also:
// - Check if the query matches a known FAQ (skip LLM call)
// - Log the interaction for quality assurance review
// - Detect escalation signals ("speak to a human", frustration)
// - Track satisfaction metrics per response

async fn chat_handler(body: ShmSlice) -> HandlerResult<VilResponse<ChatResponse>> {
    // ShmSlice: zero-copy body extraction from VIL's ExchangeHeap.
    // Critical for customer service portals during peak hours
    // (e.g., product launches, outage notifications).
    let req: ChatRequest = body.json().expect("invalid JSON body");

    // The system prompt sets the chatbot's persona. In production,
    // this would be loaded from a prompt registry and A/B tested
    // for optimal customer satisfaction scores (CSAT).
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

    // SseCollect with OpenAI dialect: the standard pattern for
    // consuming streaming LLM responses. The dialect automatically
    // handles token-by-token extraction and [DONE] detection.
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .dialect(SseDialect::openai())
        .body(body);

    // Add auth if API key is set (skip for local simulator)
    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    // Collect the full response. In a ticket deflection system,
    // the complete answer is needed before sending to the customer
    // (vs. streaming partial tokens to the UI).
    let content = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::ok(ChatResponse { content }))
}

// ── Main ────────────────────────────────────────────────────────────
// Bootstrap the customer support chatbot service.

#[tokio::main]
async fn main() {
    // Semantic types from vil_llm (compile-time validation).
    // These ensure the chatbot service correctly participates in
    // the Tri-Lane protocol for observability and fault handling.
    let _event = std::any::type_name::<LlmResponseEvent>();
    let _fault = std::any::type_name::<LlmFault>();
    let _state = std::any::type_name::<LlmUsageState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Example 23: LLM Chat (VilApp — Layer F)                  ║");
    println!("║  Semantic: LlmResponseEvent / LlmFault / LlmUsageState      ║");
    println!("║  Transport: VilApp + ServiceProcess + SseCollect            ║");
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
    println!("  Listening on http://localhost:3090/api/chat");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    // The "chat" ServiceProcess handles all customer support interactions.
    // Semantic declarations enable automatic metrics collection:
    // - LlmResponseEvent: tracks response quality and latency
    // - LlmFault: alerts on-call when the LLM backend fails
    // - LlmUsageState: monitors token consumption against budget
    let svc = ServiceProcess::new("chat")
        .prefix("/api")
        .emits::<LlmResponseEvent>()
        .faults::<LlmFault>()
        .manages::<LlmUsageState>()
        .endpoint(Method::POST, "/chat", post(chat_handler));

    // Port 3090: the customer support chatbot's internal service port.
    // In production, an API gateway (like example 002) sits in front
    // of this service to handle auth, rate limiting, and tenant routing.
    VilApp::new("llm-chat").port(3090).service(svc).run().await;
}
