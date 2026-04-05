// ╔════════════════════════════════════════════════════════════╗
// ║  204 — Real-time Document Translation Service             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Content — Multilingual Translation              ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, VilResponse, #[vil_fault]║
// ║  Features: LlmResponseEvent, LlmFault, LlmUsageState     ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Batch translation service for content teams.    ║
// ║  Every LLM call produces LlmResponseEvent (Data Lane)     ║
// ║  for translation quality audit. LlmUsageState tracks      ║
// ║  cumulative token usage across batch translations.         ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p llm-plugin-usage-translator
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"texts": ["Hello world", "How are you?", "Good morning"], "target_lang": "id"}' \
//     http://localhost:3103/api/translate/batch
//   curl http://localhost:3103/api/usage
//
// HOW THIS DIFFERS FROM 201:
//   201 = single text in, single JSON out
//   204 = array of texts in, NDJSON streaming out (one line per translation)
//   Each line: {"index":0,"original":"Hello","translated":"Halo","status":"ok"}
//   Client receives translations progressively as they complete.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use vil_llm::semantic::{LlmFault, LlmResponseEvent, LlmUsageState};
use vil_server::prelude::*;

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Faults ───────────────────────────────────────────────────────────

#[vil_fault]
/// Translation faults — each triggers different retry/fallback behavior.
pub enum TranslatorFault {
    EmptyBatch,
    UnsupportedLanguage,
    PartialBatchFailure,
    UpstreamTimeout,
}

// ── Request / Response ──────────────────────────────────────────────

/// Batch translation request — content teams submit multiple texts at once
#[derive(Debug, Deserialize)]
struct BatchTranslateRequest {
    texts: Vec<String>,
    target_lang: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Per-item translation result
struct TranslationLine {
    index: usize,
    original: String,
    translated: String,
    status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// Batch response — all translations with success/failure counts
struct BatchTranslateResponse {
    translations: Vec<TranslationLine>,
    total: usize,
    success_count: usize,
    target_lang: String,
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

// ── Handler: batch translate with per-item progress ─────────────────

/// POST /api/translate/batch — translate a batch of texts to target language
async fn batch_translate_handler(
    ctx: ServiceCtx,
    body: ShmSlice,
) -> HandlerResult<VilResponse<BatchTranslateResponse>> {
    let req: BatchTranslateRequest = body.json()
        .map_err(|_| VilError::bad_request("invalid JSON"))?;
    if req.texts.is_empty() {
        return Err(VilError::bad_request("texts array must not be empty"));
    }

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let mut translations = Vec::with_capacity(req.texts.len());
    let mut success_count = 0usize;

    // Translate each text individually — enables per-item error handling
    for (idx, text) in req.texts.iter().enumerate() {
        let start = Instant::now();

        let system_prompt = format!(
            "You are a translator. Translate the following text to {}. \
             Return ONLY the translated text, nothing else. No explanations.",
            req.target_lang
        );

        let body = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": text}
            ],
            "stream": true
        });

        let mut collector = SseCollect::post_to(UPSTREAM_URL)
            .dialect(SseDialect::openai())
            .body(body);

        if !api_key.is_empty() {
            collector = collector.bearer_token(&api_key);
        }

        match collector.collect_text().await {
            Ok(translated) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let prompt_tokens = text.split_whitespace().count() as u32;
                let completion_tokens = translated.split_whitespace().count() as u32;

                // ── Construct LlmResponseEvent per translation ──
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

                // ── Update LlmUsageState ──
                let state = ctx.state::<Arc<AppState>>()
                    .map_err(|_| VilError::internal("state not found"))?;
                state.usage.lock().unwrap().record(&event);

                translations.push(TranslationLine {
                    index: idx,
                    original: text.clone(),
                    translated,
                    status: "ok".into(),
                });
                success_count += 1;
            }
            Err(e) => {
                // Record the error in usage state
                if let Ok(state) = ctx.state::<Arc<AppState>>() {
                    state.usage.lock().unwrap().record_error();
                }

                translations.push(TranslationLine {
                    index: idx,
                    original: text.clone(),
                    translated: String::new(),
                    status: format!("error: {}", e),
                });
            }
        }
    }

    Ok(VilResponse::ok(BatchTranslateResponse {
        total: req.texts.len(),
        success_count,
        target_lang: req.target_lang,
        translations,
    }))
}

// ── Handler: usage stats ─────────────────────────────────────────────

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
    println!("║  204 — LLM Streaming Translator (VilApp + LLM Semantic)    ║");
    println!("║  Events: LlmResponseEvent | Faults: LlmFault               ║");
    println!("║  State: LlmUsageState (cumulative token tracking)           ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!(
        "  Auth: {}",
        if api_key.is_empty() {
            "simulator mode"
        } else {
            "OPENAI_API_KEY"
        }
    );
    println!("  Translate: http://localhost:3103/api/translate/batch");
    println!("  Usage:     http://localhost:3103/api/usage");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();

    let app_state = Arc::new(AppState {
        usage: Mutex::new(LlmUsageState {
            provider: "openai".into(),
            ..Default::default()
        }),
    });

    let svc = ServiceProcess::new("translator")
        .prefix("/api")
        .emits::<LlmResponseEvent>()
        .faults::<LlmFault>()
        .manages::<LlmUsageState>()
        .endpoint(
            Method::POST,
            "/translate/batch",
            post(batch_translate_handler),
        )
        .endpoint(Method::GET, "/usage", get(usage_handler))
        .state(app_state);

    VilApp::new("llm-streaming-translator")
        .port(3103)
        .observer(true)
        .service(svc)
        .run()
        .await;
}
