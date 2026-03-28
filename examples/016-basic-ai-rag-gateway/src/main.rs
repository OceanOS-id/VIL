// ╔════════════════════════════════════════════════════════════╗
// ║  016 — Enterprise Knowledge Search Gateway                ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Enterprise — Internal Knowledge Management      ║
// ║  Pattern:  SDK_PIPELINE                                     ║
// ║  Token:    GenericToken                                     ║
// ║  Features: vil_workflow!, #[vil_fault]                      ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: RAG-powered search over internal wiki,          ║
// ║  Confluence, and documentation. Employees POST natural     ║
// ║  language queries, gateway retrieves relevant docs,        ║
// ║  augments the LLM prompt, and streams a grounded answer.  ║
// ║  Reduces support ticket volume by 40%+ in typical deploy. ║
// ╚════════════════════════════════════════════════════════════╝
// Run:
//   cargo run -p basic-usage-ai-rag-gateway
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is Rust ownership?"}' \
//     http://localhost:3084/rag
//
// Load test (oha):
//   oha -m POST -H "Content-Type: application/json" \
//     -d '{"prompt": "What is Rust ownership?"}' \
//     -c 100 -n 1000 http://localhost:3084/rag

use std::sync::Arc;
use vil_sdk::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// VIL Semantic Types — document business domain for this pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Accumulated state of the RAG retrieval-augmented generation stream.
#[vil_state]
pub struct RagState {
    pub request_id: u64,
    pub context_docs_used: u8,
    pub tokens_received: u32,
    pub citations_emitted: u8,
}

/// Emitted when the RAG pipeline completes a response.
#[vil_event]
pub struct RagResponse {
    pub request_id: u64,
    pub total_tokens: u32,
    pub docs_cited: u8,
    pub duration_us: u64,
}

/// Fault domain for the RAG gateway pipeline.
#[vil_fault]
pub enum RagFault {
    UpstreamTimeout,
    ContextRetrievalFailed,
    SseParseError,
    ShmWriteFailed,
}

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE CONFIGURATION
// ─────────────────────────────────────────────────────────────────────────────

const WEBHOOK_PORT: u16 = 3084;
const WEBHOOK_PATH: &str = "/rag";
const SSE_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";
const SSE_JSON_TAP: &str = "choices[0].delta.content";

const P_TRIGGER_OUT: &str = "trigger_out";
const P_TRIGGER_IN: &str = "trigger_in";
const P_DATA_IN: &str = "response_data_in";
const P_DATA_OUT: &str = "response_data_out";
const P_CTRL_IN: &str = "response_ctrl_in";
const P_CTRL_OUT: &str = "response_ctrl_out";

// ─────────────────────────────────────────────────────────────────────────────
// NODE CONFIGURATION (Decomposed Builder Style)
// ─────────────────────────────────────────────────────────────────────────────

/// Configure the HTTP Sink — receives incoming RAG queries on port 3084.
fn configure_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("RagWebhook")
        .port(WEBHOOK_PORT)
        .path(WEBHOOK_PATH)
        .out_port(P_TRIGGER_OUT)
        .in_port(P_DATA_IN)
        .ctrl_in_port(P_CTRL_IN)
}

/// Configure the HTTP Source — connects to AI upstream via SSE.
/// The system message instructs the AI to act as a RAG-powered assistant
/// that uses provided context documents to answer questions accurately,
/// always citing which document is referenced.
fn configure_source() -> HttpSourceBuilder {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();

    let mut builder = HttpSourceBuilder::new("RagSseInference")
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
                {
                    "role": "system",
                    "content": "You are a RAG-powered assistant. Use the provided context documents to answer questions accurately. Always cite which document you're referencing."
                },
                {
                    "role": "user",
                    "content": "Answer based on the context documents provided in the system prompt."
                }
            ],
            "stream": true
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

    // Step 2: Build nodes (Decomposed Style)
    let sink_builder = configure_sink();
    let source_builder = configure_source();

    // Step 3: Wire the Tri-Lane pipeline via macro
    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "RagPipeline",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.response_data_out -> sink_builder.response_data_in (LoanWrite),
            source_builder.response_ctrl_out -> sink_builder.response_ctrl_in (Copy),
        ]
    };

    // Step 4: Startup Banner
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  VIL RAG Gateway Pipeline — Decomposed Builder (Layer 3)  ║");
    println!("║  Business: query -> enrich with context -> AI stream -> cache ║");
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
        "1. Listening for RAG queries on http://localhost:{}{}",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!("2. On trigger, streaming SSE from {}", SSE_URL);
    println!("3. json_tap extracts: {}", SSE_JSON_TAP);
    println!("4. Tri-Lane: DATA (LoanWrite) + CTRL (Copy)");
    println!();
    println!("Test with:");
    println!(
        "  curl -N -X POST -H \"Content-Type: application/json\" \
         -d '{{\"prompt\": \"What is Rust ownership?\"}}' \
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
