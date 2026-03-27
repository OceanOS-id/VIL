// ╔════════════════════════════════════════════════════════════╗
// ║  002 — API Gateway for Microservices                      ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   API Gateway / Reverse Proxy                    ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Features: ShmSlice, VilResponse, SseCollect              ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   This is the **first contact point** for all client requests in a
//   microservices architecture. The gateway receives incoming HTTP
//   requests from web/mobile clients, enriches them with auth context,
//   and forwards them to the appropriate internal service (here, an
//   LLM inference backend). In production, this pattern is used to:
//
//   - Centralize authentication (API key management)
//   - Abstract upstream service URLs from public clients
//   - Enable blue-green deployments by routing to different backends
//   - Collect usage metrics per-tenant before reaching internal services
//   - Apply rate limiting and cost budgets at the edge
//
// Architecture:
//   Client -> [This Gateway :3081] -> [LLM Service :4545]
//
// Why SseCollect?
//   The upstream LLM service streams tokens via Server-Sent Events (SSE).
//   SseCollect uses VIL's built-in async client pool — no reqwest or
//   manual Arc<Client> needed. The gateway collects the full streamed
//   response before returning a single JSON payload to the client.
//
// Run:
//   cargo run -p vil-app-gateway --release
//
// Bench:
//   oha -m POST -H "Content-Type: application/json" \
//     -d '{"prompt": "bench"}' -c 200 -n 2000 http://localhost:3081/api/trigger

use vil_server::prelude::*;
use vil_llm::semantic::{LlmResponseEvent, LlmFault, LlmUsageState};

// Upstream LLM service endpoint — in production this would be
// configured per-environment (staging vs prod) or per-tenant.
const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Request / Response ──────────────────────────────────────────────
// These types define the gateway's public API contract.
// Internal clients POST a prompt; the gateway returns the LLM's answer.

#[derive(Debug, Deserialize)]
struct TriggerRequest {
    prompt: String,
}

// VilModel derive enables zero-copy serialization through the
// ExchangeHeap — critical for high-throughput gateway scenarios
// where thousands of requests/sec flow through this single point.
#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TriggerResponse {
    content: String,
}

// ── Handler ─────────────────────────────────────────────────────────
// Gateway trigger: receives a client prompt, forwards to upstream LLM,
// collects the streamed SSE response, and returns the full answer.
// This is the core "proxy + collect" pattern used in API gateways
// that sit in front of streaming AI services.

async fn trigger_handler(
    body: ShmSlice,
) -> HandlerResult<VilResponse<TriggerResponse>> {
    // ShmSlice provides zero-copy access to the request body from
    // VIL's ExchangeHeap — avoids allocation per request at the gateway.
    let req: TriggerRequest = body.json().expect("invalid JSON body");

    // Build the upstream LLM request. The gateway decides which model
    // to use — clients never specify the model directly. This gives
    // the ops team control over cost and quality without client changes.
    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": req.prompt}
        ],
        "stream": true
    });

    // Read API key from env (empty = simulator mode, no auth needed)
    // In production, this key is injected via secrets management (Vault/K8s).
    // The gateway centralizes API key handling so individual clients
    // never need direct access to the upstream provider's credentials.
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // SseCollect: VIL's built-in SSE client that streams from upstream
    // and collects tokens into a single response. The OpenAI dialect
    // knows how to parse `data: [DONE]` and extract `choices[0].delta.content`.
    let mut collector = SseCollect::post_to(UPSTREAM_URL)
        .dialect(SseDialect::openai())
        .body(body);

    // Add auth if API key is set (skip for local simulator)
    if !api_key.is_empty() {
        collector = collector.bearer_token(&api_key);
    }

    // Collect all SSE chunks into a single string — the gateway
    // shields downstream clients from the complexity of streaming.
    // Clients get a simple JSON response instead of raw SSE events.
    let content = collector.collect_text().await
        .map_err(|e| VilError::internal(e.to_string()))?;

    Ok(VilResponse::ok(TriggerResponse { content }))
}

// ── Main ────────────────────────────────────────────────────────────
// Bootstrap the API gateway as a VIL process-oriented application.

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  001b: VilApp Gateway (SseCollect built-in client)         ║");
    println!("║  Dialect: OpenAI (data: [DONE], choices[0].delta.content)    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!("  Auth: {}", if api_key.is_empty() { "simulator mode (no auth)" } else { "OPENAI_API_KEY (Bearer)" });
    println!("  Listening on http://localhost:3081/api/trigger");
    println!("  Upstream: {} (stream: true)", UPSTREAM_URL);
    println!();

    // ServiceProcess defines the gateway as a named process in VIL's
    // process-oriented architecture. Semantic types declare what this
    // service emits/faults/manages — enabling compile-time Tri-Lane
    // validation across the entire microservices mesh.
    let svc = ServiceProcess::new("gw")
        .prefix("/api")
        .emits::<LlmResponseEvent>()     // Tri-Lane: data lane events
        .faults::<LlmFault>()            // Tri-Lane: fault lane errors
        .manages::<LlmUsageState>()      // Tri-Lane: control lane state
        .endpoint(Method::POST, "/trigger", post(trigger_handler));

    // VilApp: the process-oriented application container that provides
    // SHM pools, health endpoints, and Tri-Lane context automatically.
    // Port 3081 is the gateway's public-facing port.
    VilApp::new("vil-app-gateway")
        .port(3081)
        .service(svc)
        .run()
        .await;
}
