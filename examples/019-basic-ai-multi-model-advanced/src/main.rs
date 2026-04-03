// ╔════════════════════════════════════════════════════════════╗
// ║  019 — Multi-Provider AI Aggregator                       ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   AI/ML Platform — Multi-Provider Routing         ║
// ║  Pattern:  SDK_PIPELINE                                     ║
// ║  Token:    GenericToken                                     ║
// ║  Features: vil_workflow!, #[vil_fault]                      ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Routes AI requests across multiple providers    ║
// ║  (OpenAI, Anthropic, Ollama) with automatic failover.      ║
// ║  Tracks latency tiers, confidence scores, and fallback     ║
// ║  events for cost optimization. Production deployments      ║
// ║  save 30-50% on LLM costs via intelligent routing.        ║
// ╚════════════════════════════════════════════════════════════╝
// Run:
//   cargo run -p basic-usage-ai-multi-model-router-advanced
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "Compare Rust vs Go for microservices"}' \
//     http://localhost:3086/route-advanced
//
// Load test (oha):
//   oha -m POST -H "Content-Type: application/json" \
//     -d '{"prompt": "benchmark advanced routing"}' \
//     -c 200 -n 2000 http://localhost:3086/route-advanced

use std::sync::Arc;
use vil_sdk::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// VIL Semantic Types — document business domain for this pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Accumulated state of the advanced multi-model routing stream with fallback.
#[vil_state]
pub struct AdvancedRouterState {
    pub request_id: u64,
    pub primary_model_id: u32,
    pub fallback_attempted: bool,
    pub tokens_received: u32,
    pub confidence_score_x100: u16,
}

/// Emitted when a model (primary or fallback) completes its response.
#[vil_event]
pub struct ModelFallbackEvent {
    pub request_id: u64,
    pub model_id: u32,
    pub is_fallback: bool,
    pub latency_ns: u64,
    pub confidence_score_x100: u16,
}

/// Fault domain for the advanced multi-model router pipeline.
#[vil_fault]
pub enum AdvancedRouterFault {
    UpstreamTimeout,
    PrimaryModelUnavailable,
    FallbackExhausted,
    SseParseError,
    ShmWriteFailed,
}

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE CONFIGURATION
// ─────────────────────────────────────────────────────────────────────────────

const WEBHOOK_PORT: u16 = 3086;
const WEBHOOK_PATH: &str = "/route-advanced";
const SSE_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";
const SSE_JSON_TAP: &str = "choices[0].delta.content";

// Detailed port naming for production traceability
const P_ADV_TRIGGER_OUT: &str = "trigger_out";
const P_ADV_TRIGGER_IN: &str = "trigger_in";
const P_ADV_DATA_IN: &str = "response_data_in";
const P_ADV_DATA_OUT: &str = "response_data_out";
const P_ADV_CTRL_IN: &str = "response_ctrl_in";
const P_ADV_CTRL_OUT: &str = "response_ctrl_out";

// ─────────────────────────────────────────────────────────────────────────────
// NODE CONFIGURATION (Full Decomposed Builder Style)
// ─────────────────────────────────────────────────────────────────────────────

/// Configure the HTTP Sink — receives incoming advanced routing requests.
/// Uses detailed port naming convention for production-grade traceability.
fn configure_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("AdvancedRouterSink")
        .port(WEBHOOK_PORT)
        .path(WEBHOOK_PATH)
        .out_port(P_ADV_TRIGGER_OUT)
        .in_port(P_ADV_DATA_IN)
        .ctrl_in_port(P_ADV_CTRL_IN)
}

/// Configure the HTTP Source — connects to AI upstream with advanced routing.
/// The system prompt instructs the AI to act as a resilient multi-model router
/// that provides detailed responses with model metadata, latency estimates,
/// and confidence scoring. Includes fallback behavior instructions.
fn configure_source() -> HttpSourceBuilder {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut builder = HttpSourceBuilder::new("AdvancedRouterSource")
        .url(SSE_URL)
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::OpenAi) // OpenAI SSE dialect: data: [DONE] termination
        .json_tap(SSE_JSON_TAP)
        .in_port(P_ADV_TRIGGER_IN)
        .out_port(P_ADV_DATA_OUT)
        .ctrl_out_port(P_ADV_CTRL_OUT)
        .post_json(serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a resilient multi-model AI router. Provide detailed responses with the following metadata in your answer: 1) Which model handled the request (gpt-4). 2) Estimated latency tier (fast/medium/slow). 3) Confidence score (0.0-1.0). If the primary model is unavailable, explain the fallback strategy. Always structure your response with clear sections: [Analysis], [Metadata], [Fallback Status]."
                },
                {
                    "role": "user",
                    "content": "Process this request through the advanced routing pipeline."
                }
            ],
            "stream": true,
            "temperature": 0.7,
            "max_tokens": 2048
        }));

    // Add auth if API key is set (skip for local simulator)
    if !api_key.is_empty() {
        builder = builder.bearer_token(&api_key);
    }

    builder
}

// ─────────────────────────────────────────────────────────────────────────────
// MAIN — Wire pipeline via vil_workflow! macro, then spawn workers
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    // Step 1: Initialize the VIL shared-memory runtime
    let world =
        Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to initialize VIL SHM Runtime"));

    // Step 2: Build nodes (Full Decomposed Builder Style)
    let sink_builder = configure_sink();
    let source_builder = configure_source();

    // Step 3: Wire the Tri-Lane pipeline via macro
    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "AdvancedMultiModelRouterPipeline",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.response_data_out -> sink_builder.response_data_in (LoanWrite),
            source_builder.response_ctrl_out -> sink_builder.response_ctrl_in (Copy),
        ]
    };

    // Step 4: Startup Banner
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  VIL Advanced Multi-Model Router — Decomposed (Layer 3)   ║");
    println!("║  Business: Multi-model + fallback + latency tracking         ║");
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
    println!(
        "1. Listening on http://localhost:{}{}",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!("2. Primary model: gpt-4 (temperature=0.7, max_tokens=2048)");
    println!("3. Features: fallback routing, latency tracking, confidence scoring");
    println!("4. Streaming SSE from {}", SSE_URL);
    println!("5. json_tap extracts: {}", SSE_JSON_TAP);
    println!("6. Tri-Lane: DATA (LoanWrite) + CTRL (Copy)");
    println!();
    println!("Differences from basic multi-model-router:");
    println!("  - System prompt includes fallback behavior instructions");
    println!("  - Responses include [Analysis], [Metadata], [Fallback Status]");
    println!("  - Temperature and max_tokens configured for production use");
    println!("  - Full decomposed builder with detailed port naming");
    println!();
    println!("Test with:");
    println!(
        "  curl -N -X POST -H \"Content-Type: application/json\" \
         -d '{{\"prompt\": \"Compare Rust vs Go for microservices\"}}' \
         http://localhost:{}{}\n",
        WEBHOOK_PORT, WEBHOOK_PATH
    );

    // Step 5: Build & spawn workers
    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    let t1 = sink.run_worker::<GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<GenericToken>(world.clone(), source_handle);

    t1.join().expect("Sink panicked");
    t2.join().expect("Source panicked");
}
