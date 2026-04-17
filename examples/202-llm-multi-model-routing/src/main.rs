// ╔══════════��═════════════════════��═══════════════════════════╗
// ║  202 — Multi-Model Translation Service                    ║
// ╠═════���══════════════════════════════════════════════════════╣
// ║  Domain:   Localization — Multi-Language Translation        ║
// ║  Pattern:  VX_APP                                           ║
// ║  Features: LlmRouter (RoundRobin), multiple active models  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Translation API that round-robins across models ║
// ║  for load distribution. Both models active and used.       ║
// ║  Track per-model usage for capacity planning.              ║
// ╚═��══════════════════════════════════════════════════════════╝
//
// Requires: ai-endpoint-simulator at localhost:4545
//
// Run:   cargo run -p vil-llm-multi-model-routing
// Test:
//   curl -X POST http://localhost:8080/api/translate/translate \
//     -H 'Content-Type: application/json' \
//     -d '{"text":"Hello, how are you?","target_lang":"Indonesian"}'

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use vil_llm::{ChatMessage, LlmProvider, LlmRouter, OpenAiConfig, OpenAiProvider, RouterStrategy};
use vil_server::prelude::*;

#[derive(Debug, Deserialize)]
struct TranslateRequest {
    text: String,
    target_lang: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
struct TranslateResponse {
    original: String,
    translated: String,
    target_lang: String,
    model_used: String,
}

struct TranslateState {
    router: LlmRouter,
    total: AtomicU64,
}

async fn translate(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<TranslateResponse>> {
    let req: TranslateRequest = body.json()
        .map_err(|e| VilError::bad_request(format!("invalid JSON: {}", e)))?;

    let state = ctx.state::<Arc<TranslateState>>()
        .map_err(|_| VilError::internal("state not found"))?;

    state.total.fetch_add(1, Ordering::Relaxed);

    let messages = vec![
        ChatMessage::system(&format!(
            "Translate the following text to {}. Return ONLY the translation, nothing else.",
            req.target_lang
        )),
        ChatMessage::user(&req.text),
    ];

    // RoundRobin router — distributes load across both models
    let response = state.router.chat(&messages).await
        .map_err(|e| VilError::internal(format!("translation failed: {}", e)))?;

    Ok(VilResponse::ok(TranslateResponse {
        original: req.text,
        translated: response.content,
        target_lang: req.target_lang,
        model_used: response.model,
    }))
}

#[tokio::main]
async fn main() {
    let upstream = std::env::var("LLM_UPSTREAM")
        .unwrap_or_else(|_| "http://127.0.0.1:4545".into());
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let model_a = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-4").base_url(&format!("{}/v1", upstream)),
    ));
    let model_b = Arc::new(OpenAiProvider::new(
        OpenAiConfig::new(&api_key, "gpt-3.5-turbo").base_url(&format!("{}/v1", upstream)),
    ));

    // RoundRobin: both models active, alternating requests
    let router = LlmRouter::new(RouterStrategy::RoundRobin)
        .add_provider(model_a)
        .add_provider(model_b);

    let state = Arc::new(TranslateState {
        router,
        total: AtomicU64::new(0),
    });

    let svc = ServiceProcess::new("translate")
        .endpoint(Method::POST, "/translate", post(translate))
        .state(state);

    VilApp::new("multi-model-translation")
        .port(8080)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
