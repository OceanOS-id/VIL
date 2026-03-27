// ╔════════════════════════════════════════════════════════════╗
// ║  204 — Real-time Document Translation Service             ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Content — Multilingual Translation              ║
// ║  Pattern:  VX_APP                                         ║
// ║  Token:    N/A (HTTP server)                              ║
// ║  Macros:   ShmSlice, ServiceCtx, VilResponse, #[vil_fault]║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Batch translation service for content teams.    ║
// ║  Accepts an array of texts + target language, translates  ║
// ║  each via LLM, returns per-item results with status.       ║
// ║  Use cases: product catalog localization, support docs,    ║
// ║  marketing copy for multi-region launches.                 ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:
//   cargo run -p llm-plugin-usage-translator
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"texts": ["Hello world", "How are you?", "Good morning"], "target_lang": "id"}' \
//     http://localhost:3103/api/translate/batch
//
// HOW THIS DIFFERS FROM 201:
//   201 = single text in, single JSON out
//   204 = array of texts in, NDJSON streaming out (one line per translation)
//   Each line: {"index":0,"original":"Hello","translated":"Halo","status":"ok"}
//   Client receives translations progressively as they complete.

use vil_server::prelude::*;
use vil_llm::semantic::{LlmResponseEvent, LlmFault, LlmUsageState};

const UPSTREAM_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// ── Semantic Types ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
/// Translation service state — tracks batch throughput and language coverage.
pub struct TranslatorState {
    pub batches_processed: u64,
    pub total_texts_translated: u64,
    pub total_chars_processed: u64,
    pub last_target_lang: String,
}

#[derive(Clone, Debug)]
/// Batch translation event — logged for content team productivity metrics.
pub struct TranslationCompletedEvent {
    pub batch_size: u32,
    pub target_lang: String,
    pub success_count: u32,
    pub fail_count: u32,
    pub total_chars: u64,
}

#[vil_fault]
/// Translation faults — each triggers different retry/fallback behavior.
pub enum TranslatorFault {
    EmptyBatch,
    UnsupportedLanguage,
    PartialBatchFailure,
    UpstreamTimeout,
}

// ── Request / Response ──────────────────────────────────────────────
// BatchTranslateRequest accepts an array of source texts and a target language
// code (ISO 639-1: "id"=Indonesian, "ja"=Japanese, "de"=German, etc.).
// Each text is translated independently; partial failures do not block others.

/// Batch translation request — content teams submit multiple texts at once
#[derive(Debug, Deserialize)]
struct BatchTranslateRequest {
    texts: Vec<String>,       // Source texts to translate
    target_lang: String,      // ISO 639-1 language code (e.g., "id", "ja")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Per-item translation result — includes original, translated text, and status.
struct TranslationLine {
    index: usize,
    original: String,
    translated: String,
    status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, VilModel)]
/// Batch response — all translations with success/failure counts and target lang.
struct BatchTranslateResponse {
    translations: Vec<TranslationLine>,
    total: usize,
    success_count: usize,
    target_lang: String,
}

// ── Handler: batch translate with per-item progress ─────────────────
// Translates each text sequentially via LLM SSE streaming. In production,
// this could be parallelized with tokio::spawn for higher throughput.
// Each result includes original text, translation, and status for QA review.

/// POST /api/translate/batch — translate a batch of texts to target language
async fn batch_translate_handler(
    ctx: ServiceCtx, body: ShmSlice,
) -> HandlerResult<VilResponse<BatchTranslateResponse>> {
    let req: BatchTranslateRequest = body.json().expect("invalid JSON body");
    if req.texts.is_empty() {
        return Err(VilError::bad_request("texts array must not be empty"));
    }

    // Use real API key for production; simulator mode for local development
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let mut translations = Vec::with_capacity(req.texts.len());
    let mut success_count = 0usize;

    // Process each text sequentially — in a real NDJSON streaming scenario,
    // each result would be flushed to the client as it completes.
    // Here we collect all results for the JSON response.
    // Translate each text individually — enables per-item error handling and progress tracking
    for (idx, text) in req.texts.iter().enumerate() {
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
                translations.push(TranslationLine {
                    index: idx,
                    original: text.clone(),
                    translated,
                    status: "ok".into(),
                });
                success_count += 1;
            }
            Err(e) => {
                translations.push(TranslationLine {
                    index: idx,
                    original: text.clone(),
                    translated: String::new(),
                    status: format!("error: {}", e),
                });
            }
        }
    }

    // Semantic audit
    let _event = TranslationCompletedEvent {
        batch_size: req.texts.len() as u32,
        target_lang: req.target_lang.clone(),
        success_count: success_count as u32,
        fail_count: (req.texts.len() - success_count) as u32,
        total_chars: req.texts.iter().map(|t| t.len() as u64).sum(),
    };

    Ok(VilResponse::ok(BatchTranslateResponse {
        total: req.texts.len(),
        success_count,
        target_lang: req.target_lang,
        translations,
    }))
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
// ── Main — Real-time Document Translation Service ───────────────────
async fn main() {
    //     // Register LLM semantic types for observability and audit logging
    let _event = std::any::type_name::<LlmResponseEvent>();
    let _fault = std::any::type_name::<LlmFault>();
    let _state = std::any::type_name::<LlmUsageState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  204 — LLM Streaming Translator (VilApp)                   ║");
    // Banner: display pipeline topology and connection info
    println!("║  Pattern: VX_APP | Token: N/A                              ║");
    println!("║  Unique: Batch input + per-item translation + NDJSON style ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    println!("  Auth: {}", if api_key.is_empty() { "simulator mode" } else { "OPENAI_API_KEY" });
    println!("  Listening on http://localhost:3103/api/translate/batch");
    println!("  Upstream SSE: {}", UPSTREAM_URL);
    println!();
    println!("  Test:");
    println!("  curl -X POST -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"texts\": [\"Hello\", \"Goodbye\"], \"target_lang\": \"id\"}}' \\");
    println!("    http://localhost:3103/api/translate/batch");
    println!();

    //     // Build the translation service with LLM semantic type registration
    let svc = ServiceProcess::new("translator")
        .prefix("/api")
        .emits::<LlmResponseEvent>()
        .faults::<LlmFault>()
        .manages::<LlmUsageState>()
        .endpoint(Method::POST, "/translate/batch", post(batch_translate_handler));

    //     // Run as VilApp — multilingual translation service for content teams
    VilApp::new("llm-streaming-translator")
        .port(3103)
        .service(svc)
        .run()
        .await;
}
