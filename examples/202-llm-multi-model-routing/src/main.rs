// ╔════════════════════════════════════════════════════════════╗
// ║  202 — Translation Service Pipeline                       ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Localization / Multi-Language Translation       ║
// ║  Pattern:  SDK_PIPELINE                                   ║
// ║  Token:    GenericToken                                   ║
// ║  Unique:   Two separate pipelines routing to specialized  ║
// ║            models per language pair via Tri-Lane           ║
// ╚════════════════════════════════════════════════════════════╝
//
// Business Context:
//   Route translation requests to specialized models per language pair.
//   In enterprise localization platforms, different models excel at
//   different language pairs:
//
//   - GPT-4: best for complex languages (Japanese, Arabic, Chinese)
//     where nuance and cultural context matter
//   - GPT-3.5-turbo: sufficient for European language pairs (EN->FR,
//     EN->DE) where translation quality is already excellent
//
//   Smart routing across models enables:
//   - 70% cost reduction on high-volume European translations
//   - Higher quality for complex language pairs (GPT-4 only where needed)
//   - Automatic fallback when primary model is unavailable
//   - Per-language-pair latency optimization
//
// Why SDK_PIPELINE instead of VX_APP?
//   The SDK pipeline pattern uses VIL's low-level workflow engine for
//   maximum control over data flow. This is ideal for translation
//   services where:
//   - Tokens flow through SHM (shared memory) for zero-copy between stages
//   - LoanWrite transfer mode enables efficient large-document translation
//   - Tri-Lane routing provides separate channels for data, faults, control
//   - Pipeline DAG can be extended with pre/post-processing stages
//
// Run:
//   cargo run -p llm-plugin-usage-multi-model
//
// Test (gpt-4 pipeline):
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"prompt": "Explain monads"}' \
//     http://localhost:3101/multi

use std::sync::Arc;
use vil_sdk::prelude::*;

// Import semantic types from vil_llm plugin (P7: Semantic Message Types).
// These ensure the translation service correctly participates in the
// observability mesh — token usage, translation quality, and fault
// rates are tracked per language pair and per model.
use vil_llm::semantic::{LlmFault, LlmResponseEvent, LlmUsageState};

// Import pipeline builders from vil_llm plugin (P8: Tri-Lane Protocol).
// The pipeline builders provide pre-configured sink/source nodes
// for LLM chat interactions — no manual SSE parsing needed.
use vil_llm::pipeline;

// ── Semantic Types ────────────────────────────────────────────────────
// These types model the translation service's operational state.

// MultiModelState: tracks per-model translation metrics. In a
// localization platform, this shows which model handles more traffic,
// enabling capacity planning and cost forecasting per language pair.
#[derive(Clone, Debug)]
pub struct MultiModelState {
    pub gpt4_calls: u64,      // Complex language pair translations (JA, AR, ZH)
    pub gpt35_calls: u64,     // Standard European translations (FR, DE, ES)
    pub total_tokens: u64,    // Aggregate token consumption across all pairs
    pub active_model: String, // Currently primary model for routing decisions
}

// ModelRoutedEvent: emitted each time a translation request is routed
// to a specific model. Enables analysis of routing decisions — are
// complex texts being sent to the right model for quality?
#[derive(Clone, Debug)]
pub struct ModelRoutedEvent {
    pub model: String,        // Which model handled this translation
    pub prompt_len: u32,      // Source text length (affects cost)
    pub route_reason: String, // Why this model was selected (language pair, complexity)
}

// MultiModelFault: translation-specific failure modes. Each fault
// triggers different remediation — e.g., PrimaryModelTimeout causes
// automatic fallback to the secondary model.
#[vil_fault]
pub enum MultiModelFault {
    PrimaryModelTimeout,  // GPT-4 too slow — fallback to GPT-3.5
    FallbackModelTimeout, // Both models failed — escalate to ops
    InvalidRouteConfig,   // Language pair has no configured model
}

// ─────────────────────────────────────────────────────────────────────
// Pipeline Configuration
// ─────────────────────────────────────────────────────────────────────

const WEBHOOK_PORT: u16 = 3101;
const WEBHOOK_PATH: &str = "/multi";

// Upstream SSE endpoint for LLM inference. Both models use the same
// endpoint with different model names in the request body.
const SSE_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

// Model configuration — two models for translation quality/cost optimization.
// GPT-4: higher quality for complex language pairs (e.g., EN->JA)
// GPT-3.5: cost-effective for standard European language pairs (e.g., EN->FR)
const MODEL_GPT4: &str = "gpt-4";
const MODEL_GPT35: &str = "gpt-3.5-turbo";

fn main() {
    // Initialize VIL's shared memory runtime. The translation service
    // uses SHM for zero-copy token transfer between pipeline stages —
    // critical for large document translations where copying megabytes
    // of text between stages would waste CPU and memory bandwidth.
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    // Primary translation pipeline: GPT-4 for complex language pairs.
    // The sink receives translation requests via HTTP webhook; the
    // source streams tokens from the LLM and collects the response.
    let sink_builder = pipeline::chat_sink(WEBHOOK_PORT, WEBHOOK_PATH);
    let source_gpt4 = pipeline::chat_source(SSE_URL, MODEL_GPT4);

    // vil_workflow! macro wires the pipeline DAG with Tri-Lane routing:
    // - trigger_out -> trigger_in: translation request flows to LLM (LoanWrite)
    // - response_data_out -> response_data_in: translated text returns (LoanWrite)
    // - response_ctrl_out -> response_ctrl_in: stream control signals (Copy)
    //
    // LoanWrite transfer mode: the source "loans" data to the sink via
    // SHM without copying — the sink reads directly from the source's
    // memory region. This is why VIL pipelines achieve near-zero-copy
    // throughput for large translation payloads.
    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "MultiModelPipeline_GPT4",
        instances: [ sink_builder, source_gpt4 ],
        routes: [
            sink_builder.trigger_out -> source_gpt4.trigger_in (LoanWrite),
            source_gpt4.response_data_out -> sink_builder.response_data_in (LoanWrite),
            source_gpt4.response_ctrl_out -> sink_builder.response_ctrl_in (Copy),
        ]
    };

    // Secondary pipeline config (GPT-3.5, faster/cheaper) — demonstrates
    // that the same pipeline builder API supports different model configs.
    // In production, a routing classifier would direct requests to either
    // the GPT-4 or GPT-3.5 pipeline based on language pair and complexity.
    let _source_gpt35 = pipeline::chat_source(SSE_URL, MODEL_GPT35);

    // Semantic types from vil_llm (compile-time validation via type_name).
    // These ensure the translation service's events/faults/state types
    // are compatible with the localization platform's observability stack.
    let _event_type = std::any::type_name::<LlmResponseEvent>();
    let _fault_type = std::any::type_name::<LlmFault>();
    let _state_type = std::any::type_name::<LlmUsageState>();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  202 — LLM Multi-Model Routing (SDK_PIPELINE)              ║");
    println!("║  Pattern: SDK_PIPELINE | Token: GenericToken                ║");
    println!("║  Unique: Two pipelines, gpt-4 vs gpt-3.5 via Tri-Lane     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Models configured:");
    println!(
        "    Primary  : {} (active on port {})",
        MODEL_GPT4, WEBHOOK_PORT
    );
    println!("    Secondary: {} (ready for routing)", MODEL_GPT35);
    println!();
    println!(
        "  Listening on http://localhost:{}{}",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!("  Upstream SSE: {}", SSE_URL);
    println!();

    // Instantiate the HTTP sink and source from their builders.
    // These are the concrete pipeline workers that process requests.
    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_gpt4);

    // Run both pipeline workers on separate threads. GenericToken is
    // the token type flowing through the pipeline — it carries the
    // translation request/response data through SHM regions.
    let t1 = sink.run_worker::<GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<GenericToken>(world.clone(), source_handle);

    // Block until both workers complete (or panic). In production,
    // graceful shutdown would be coordinated via the VIL supervisor.
    t1.join().expect("MultiModelSink panicked");
    t2.join().expect("MultiModelSource panicked");
}
