// ╔════════════════════════════════════════════════════════════╗
// ║  018 — AI Model Cost Optimizer (Multi-Model Router)       ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   AI Infrastructure — Cost Management             ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: LlmRouter, RouterStrategy, ServiceCtx,         ║
// ║            ShmSlice, VilResponse, cost-aware routing       ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Route AI requests to the cheapest model that    ║
// ║  meets quality threshold. Classify prompt complexity:       ║
// ║    - Simple Q&A (short prompt) → GPT-3.5 ($0.50/1M tok)   ║
// ║    - Complex analysis (long prompt) → GPT-4 ($30/1M tok)  ║
// ║  Smart routing reduces AI costs by 40-60% at scale.        ║
// ╚════════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator running at localhost:4545
//
// Run:   cargo run -p vil-basic-ai-multi-model-router
// Test:
//   curl -X POST http://localhost:8080/api/router/route \
//     -H 'Content-Type: application/json' \
//     -d '{"prompt":"What is Rust?","max_cost_usd":0.001}'
//   curl http://localhost:8080/api/router/models
//   curl http://localhost:8080/api/router/stats

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vil_llm::{
    ChatMessage, LlmProvider, LlmRouter, OpenAiConfig, OpenAiProvider, RouterStrategy,
};
use vil_server::prelude::*;

// ── Models ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RouteRequest {
    prompt: String,
    #[serde(default)]
    max_cost_usd: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct RouteResponse {
    content: String,
    model_used: String,
    tier: String,
    estimated_cost_usd: f64,
    prompt_tokens_approx: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct ModelInfo {
    name: String,
    tier: String,
    cost_per_1m_tokens: f64,
    best_for: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct RouterStats {
    total_requests: u64,
    gpt4_requests: u64,
    gpt35_requests: u64,
    estimated_savings_pct: f64,
}

// ── State ────────────────────────────────────────────────────────────────

struct RouterState {
    router_gpt4: Arc<dyn LlmProvider>,
    router_gpt35: Arc<dyn LlmProvider>,
    fallback_router: LlmRouter,
    total: AtomicU64,
    gpt4_count: AtomicU64,
    gpt35_count: AtomicU64,
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// POST /route — Route prompt to optimal model based on complexity + budget.
async fn route_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<RouteResponse>> {
    let req: RouteRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;

    let state = ctx.state::<Arc<RouterState>>()
        .map_err(|_| VilError::internal("router state not found"))?;

    state.total.fetch_add(1, Ordering::Relaxed);

    // ── Classify prompt complexity ──
    let prompt_tokens = req.prompt.split_whitespace().count();
    let is_complex = prompt_tokens > 50
        || req.prompt.contains("analyze")
        || req.prompt.contains("compare")
        || req.prompt.contains("explain in detail");

    // ── Budget check ──
    let budget = req.max_cost_usd.unwrap_or(1.0);
    let force_cheap = budget < 0.001;

    // ── Route decision ──
    let (provider, tier, cost_rate): (&dyn LlmProvider, &str, f64) =
        if is_complex && !force_cheap {
            state.gpt4_count.fetch_add(1, Ordering::Relaxed);
            (state.router_gpt4.as_ref(), "premium", 30.0)
        } else {
            state.gpt35_count.fetch_add(1, Ordering::Relaxed);
            (state.router_gpt35.as_ref(), "economy", 0.50)
        };

    // ── Call LLM via vil_llm ──
    let messages = vec![
        ChatMessage::system("You are a helpful AI assistant. Be concise."),
        ChatMessage::user(&req.prompt),
    ];

    let response = provider.chat(&messages).await
        .map_err(|e| VilError::internal(format!("LLM call failed: {}", e)))?;

    let estimated_cost = (prompt_tokens as f64 / 1_000_000.0) * cost_rate;

    Ok(VilResponse::ok(RouteResponse {
        content: response.content,
        model_used: provider.model().to_string(),
        tier: tier.into(),
        estimated_cost_usd: estimated_cost,
        prompt_tokens_approx: prompt_tokens,
    }))
}

/// GET /models — List available models with pricing.
async fn list_models() -> VilResponse<Vec<ModelInfo>> {
    VilResponse::ok(vec![
        ModelInfo {
            name: "gpt-4".into(),
            tier: "premium".into(),
            cost_per_1m_tokens: 30.0,
            best_for: "Complex analysis, code review, reasoning".into(),
        },
        ModelInfo {
            name: "gpt-3.5-turbo".into(),
            tier: "economy".into(),
            cost_per_1m_tokens: 0.50,
            best_for: "Simple Q&A, summaries, classification".into(),
        },
    ])
}

/// GET /stats — Routing statistics and cost savings.
async fn stats(ctx: ServiceCtx) -> HandlerResult<VilResponse<RouterStats>> {
    let state = ctx.state::<Arc<RouterState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    let total = state.total.load(Ordering::Relaxed);
    let gpt4 = state.gpt4_count.load(Ordering::Relaxed);
    let gpt35 = state.gpt35_count.load(Ordering::Relaxed);

    // If everything went to GPT-4, cost would be 100%.
    // Savings = percentage routed to cheaper model.
    let savings = if total > 0 {
        (gpt35 as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Ok(VilResponse::ok(RouterStats {
        total_requests: total,
        gpt4_requests: gpt4,
        gpt35_requests: gpt35,
        estimated_savings_pct: savings,
    }))
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let upstream = std::env::var("LLM_UPSTREAM")
        .unwrap_or_else(|_| "http://127.0.0.1:4545".into());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    // Two providers — same upstream (simulator), different model params
    let gpt4 = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-4")
            .base_url(&format!("{}/v1", upstream)),
    ));
    let gpt35 = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-3.5-turbo")
            .base_url(&format!("{}/v1", upstream)),
    ));

    // Fallback router: try GPT-4 first, fallback to GPT-3.5 on error
    let fallback = LlmRouter::new(RouterStrategy::Fallback)
        .add_provider(gpt4.clone())
        .add_provider(gpt35.clone());

    let state = Arc::new(RouterState {
        router_gpt4: gpt4,
        router_gpt35: gpt35,
        fallback_router: fallback,
        total: AtomicU64::new(0),
        gpt4_count: AtomicU64::new(0),
        gpt35_count: AtomicU64::new(0),
    });

    let svc = ServiceProcess::new("router")
        .endpoint(Method::POST, "/route", post(route_handler))
        .endpoint(Method::GET, "/models", get(list_models))
        .endpoint(Method::GET, "/stats", get(stats))
        .state(state);

    VilApp::new("ai-multi-model-router")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
