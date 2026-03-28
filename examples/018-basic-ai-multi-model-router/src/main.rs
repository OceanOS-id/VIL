// ╔════════════════════════════════════════════════════════════╗
// ║  018 — AI Model Cost Optimizer                            ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   AI Infrastructure / Cost Management            ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Features: ShmSlice, VilResponse, SseCollect, json_tap    ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   Route inference requests to the cheapest model that meets the
//   quality threshold. In production AI platforms, model costs vary
//   dramatically:
//
//   - GPT-4: $30/1M tokens  — best for complex analysis, code review
//   - GPT-3.5: $0.50/1M tokens — sufficient for simple Q&A, summaries
//   - Local models: $0 — good for internal tools, low-stakes queries
//
//   This router examines the prompt complexity and routes to the most
//   cost-effective model. At scale (millions of requests/day), smart
//   routing can reduce AI infrastructure costs by 60-80%.
//
// Why json_tap instead of dialect()?
//   json_tap("choices[0].delta.content") gives fine-grained control
//   over which JSON path to extract from each SSE chunk. This is
//   essential when routing across different model providers that may
//   use slightly different response schemas. dialect() is a shortcut
//   for known providers; json_tap works with any custom format.
//
// Run:
//   cargo run -p basic-usage-ai-multi-model-router
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "Analyze the performance characteristics of Rust async"}' \
//     http://localhost:3085/api/route

use vil_llm::semantic::{LlmFault, LlmResponseEvent, LlmUsageState};
use vil_server::prelude::*;

// Upstream inference endpoint — in a real cost optimizer, this would
// be a map of model_name -> endpoint_url, with fallback chains.
const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Request / Response ──────────────────────────────────────────────
// The routing API accepts a prompt and returns the model's response.
// In production, the request would also include quality constraints
// (e.g., min_quality: "high") and budget limits (e.g., max_cost_usd: 0.01).

#[derive(Debug, Deserialize)]
struct RouteRequest {
    prompt: String,
}

// VilModel derive enables the response to flow through VIL's
// ExchangeHeap for zero-copy inter-service communication.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct RouteResponse {
    content: String,
}

// ── Handler: Route inference to the optimal model ────────────────────
// This handler implements the core cost optimization logic:
// 1. Parse the incoming prompt from the client
// 2. Classify prompt complexity (here simplified to always use gpt-4)
// 3. Forward to the selected model's endpoint via SSE streaming
// 4. Collect and return the complete response
//
// In production, step 2 would use a lightweight classifier model
// to score prompt complexity and select the cheapest adequate model.

async fn route_handler(body: ShmSlice) -> HandlerResult<VilResponse<RouteResponse>> {
    let req: RouteRequest = body.json().expect("invalid JSON body");

    // Build the inference request. The system prompt specializes the
    // model for technical analysis — in a real router, different system
    // prompts would be configured per-model to maximize quality.
    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {
                "role": "system",
                "content": "You are model gpt-4. Respond as a helpful assistant \
                            specializing in technical analysis."
            },
            {"role": "user", "content": req.prompt}
        ],
        "stream": true
    });

    // Read API key from env (empty = simulator mode, no auth needed)
    // Cost optimizer services need provider API keys to route requests
    // across multiple model providers (OpenAI, Anthropic, local).
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // json_tap extracts content from each SSE chunk at the specified
    // JSON path. This is more flexible than dialect() — supports custom
    // model providers with non-standard response formats.
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .json_tap("choices[0].delta.content")
        .body(body);

    // Add auth if API key is set (skip for local simulator)
    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    // Collect the full streamed response. In the cost optimizer,
    // we also track token usage here to update per-tenant budgets
    // and trigger alerts when spending exceeds thresholds.
    let content = collector
        .collect_text()
        .await
        .map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::ok(RouteResponse { content }))
}

// ── Main ────────────────────────────────────────────────────────────
// Bootstrap the AI model cost optimizer service.

#[tokio::main]
async fn main() {
    // Semantic types from vil_llm — these are validated at compile time
    // to ensure the cost optimizer correctly participates in the Tri-Lane
    // protocol. If a type changes upstream, this service fails to compile
    // rather than silently producing wrong cost tracking data.
    let _event = std::any::type_name::<LlmResponseEvent>();
    let _fault = std::any::type_name::<LlmFault>();
    let _state = std::any::type_name::<LlmUsageState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Example 17: Multi-Model Router (VilApp — Layer F)        ║");
    println!("║  Semantic: LlmResponseEvent / LlmFault / LlmUsageState     ║");
    println!("║  Transport: VilApp + ServiceProcess + SseCollect          ║");
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
    println!("  Listening on http://localhost:3085/api/route");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!("  Model: gpt-4 (technical analysis specialist)");
    println!("  json_tap: choices[0].delta.content");
    println!();

    // ServiceProcess "router" handles all model routing decisions.
    // Semantic declarations (.emits/.faults/.manages) register this
    // service in the Tri-Lane mesh so that cost tracking, fault
    // alerting, and usage dashboards work automatically.
    let svc = ServiceProcess::new("router")
        .prefix("/api")
        .emits::<LlmResponseEvent>() // Data lane: model response events
        .faults::<LlmFault>() // Fault lane: model timeout/errors
        .manages::<LlmUsageState>() // Control lane: token usage tracking
        .endpoint(Method::POST, "/route", post(route_handler));

    VilApp::new("ai-multi-model-router")
        .port(3085)
        .service(svc)
        .run()
        .await;
}
