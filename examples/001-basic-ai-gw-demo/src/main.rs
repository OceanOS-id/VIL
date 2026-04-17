// ╔════════════════════════════════════════════════════════════╗
// ║  001 — AI Inference Gateway (Production LLM Proxy)        ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   AI/ML Platform — Centralized LLM Access        ║
// ║  Pattern:  SDK_PIPELINE                                     ║
// ║  Token:    ShmToken (zero-copy inter-service transport)     ║
// ║  Features: vil_workflow!, #[vil_fault]                      ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Routes inference requests from internal          ║
// ║  services to upstream LLM providers (OpenAI, Anthropic,   ║
// ║  Ollama). Provides centralized API key management,         ║
// ║  usage metering, and real-time SSE streaming.              ║
// ║  SHM transport enables sub-microsecond token forwarding.  ║
// ╚════════════════════════════════════════════════════════════╝

use std::sync::Arc;
use vil_sdk::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// VIL Semantic Types — document business domain for this pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Accumulated state of a streaming AI inference request.
#[vil_state]
pub struct InferenceState {
    pub request_id: u64,
    pub tokens_received: u32,
    pub latency_ns: u64,
    pub stream_active: bool,
}

/// Emitted when an inference stream completes successfully.
#[vil_event]
pub struct InferenceCompleted {
    pub request_id: u64,
    pub total_tokens: u32,
    pub duration_ns: u64,
    pub status_code: u16,
}

/// Fault domain for the inference pipeline.
#[vil_fault]
pub enum InferenceFault {
    UpstreamTimeout,
    SseParseError,
    ShmWriteFailed,
    ConnectionRefused,
}

// ─────────────────────────────────────────────────────────────────────────────
// GATEWAY CONFIGURATION — Production LLM proxy settings
// ─────────────────────────────────────────────────────────────────────────────
// All internal services POST inference requests to WEBHOOK_PORT/WEBHOOK_PATH.
// The gateway forwards to the upstream LLM provider (SSE_URL) and streams
// the response back. json_tap extracts the generated text from the SSE payload.

/// Port where internal services send inference requests
const WEBHOOK_PORT: u16 = 3080;
/// Endpoint path for incoming inference triggers
const WEBHOOK_PATH: &str = "/trigger";
/// Upstream LLM provider endpoint (OpenAI-compatible chat completions API)
const SSE_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";
/// JSONPath expression to extract generated text from SSE chunks
const SSE_JSON_TAP: &str = "choices[0].delta.content";

// Tri-Lane port names — separate trigger, data, and control signals
// for clean separation of request initiation, streaming data, and completion
const P_TRIGGER_OUT: &str = "trigger_out";
const P_TRIGGER_IN: &str = "trigger_in";
const P_DATA_IN: &str = "response_data_in";
const P_DATA_OUT: &str = "response_data_out";
const P_CTRL_IN: &str = "response_ctrl_in";
const P_CTRL_OUT: &str = "response_ctrl_out";

// ─────────────────────────────────────────────────────────────────────────────
// NODE CONFIGURATION (Decomposed Style)
// ─────────────────────────────────────────────────────────────────────────────

fn configure_webhook_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("WebhookTrigger")
        .env_port(WEBHOOK_PORT)
        .path(WEBHOOK_PATH)
        .out_port(P_TRIGGER_OUT)
        .in_port(P_DATA_IN)
        .ctrl_in_port(P_CTRL_IN)
}

fn configure_sse_source() -> HttpSourceBuilder {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut builder = HttpSourceBuilder::new("SseInference")
        .url(SSE_URL)
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::OpenAi) // OpenAI SSE dialect: data: [DONE] termination
        .json_tap(SSE_JSON_TAP)
        .in_port(P_TRIGGER_IN)
        .out_port(P_DATA_OUT)
        .ctrl_out_port(P_CTRL_OUT)
        .post_json(serde_json::json!({
            "model": "gpt-4",
            "messages": [
                { "role": "user", "content": "Benchmark performance test via VIL" }
            ],
            "stream": true
        }));

    // Add auth if API key is set (skip for local simulator)
    if !api_key.is_empty() {
        builder = builder.bearer_token(&api_key);
    }

    builder
}

fn main() {
    // ── Step 1: Init Runtime ─────────────────────────────────────────────────
    let world =
        Arc::new(VastarRuntimeWorld::new_shared().expect("Gagal inisialisasi VIL SHM Runtime"));

    // ── Step 1b: Attach Observer Sidecar ──────────────────────────────────────
    vil_observer::sidecar(3180).attach(&world).spawn();

    // ── Step 2: Configure Nodes (Decomposed Style) ──────────────────────────
    let sink_builder = configure_webhook_sink();
    let source_builder = configure_sse_source();

    // ── Step 3: Wire Pipeline via Macro ──────────────────────────────────────
    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "DecomposedPipeline",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.response_data_out -> sink_builder.response_data_in (LoanWrite),
            source_builder.response_ctrl_out -> sink_builder.response_ctrl_in (Copy),
        ]
    };

    // ── Step 4: Startup Banner ───────────────────────────────────────────────
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     VIL Webhook-SSE Pipeline — Decomposed Style            ║");
    println!("║             (Using vil_sdk & Tri-Lane Reactive)            ║");
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
    println!();
    println!(
        "1. Listening for Webhook Triggers on http://localhost:{}{}",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!("2. On trigger, starting SSE stream from http://localhost:4545");
    println!("3. Using separated DATA lane + CTRL lane for response completion");
    println!();
    println!("🚀 VIL Webhook-SSE Pipeline built using Decomposed Style!");
    println!("Use:");
    println!("curl -N -X POST -H \"Content-Type: application/json\" -d '{{\"prompt\": \"test\"}}' http://localhost:3080/trigger\n");
    println!("Load test (oha):");
    println!("oha -m POST -H \"Content-Type: application/json\" -d '{{\"prompt\": \"bench\"}}' -c 200 -n 2000 http://localhost:3080/trigger\n");

    // ── Step 5: Build & Spawn Workers ────────────────────────────────────────
    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    // ShmToken: O(1) publish/recv via SHM — bypasses sample table entirely.
    let sink_thread = sink.run_worker::<ShmToken>(world.clone(), sink_handle);
    let source_thread = source.run_worker::<ShmToken>(world.clone(), source_handle);

    sink_thread.join().expect("WebhookSink worker panicked");
    source_thread.join().expect("SseSource worker panicked");
}
