// ╔════════════════════════════════════════════════════════════╗
// ║  101b — Multi-Pipeline Benchmark (ShmToken)               ║
// ╠════════════════════════════════════════════════════════════╣
// ║  3-node chain: Webhook → Transform → SSE Upstream         ║
// ║  Pattern: SDK_PIPELINE with ShmToken (zero-copy)          ║
// ║  Purpose: head-to-head multi-pipeline vs VilApp            ║
// ║                                                            ║
// ║  Topology:                                                 ║
// ║    Client → HttpSink(:3090/trigger) → [Transform] →       ║
// ║    HttpSource(→ upstream:4545/v1/chat/completions) →       ║
// ║    Response back to client                                 ║
// ║                                                            ║
// ║  The transform stage adds metadata (timestamp, node_id)   ║
// ║  to prove data flows through all 3 stages.                ║
// ╚════════════════════════════════════════════════════════════╝

use std::sync::Arc;
use vil_sdk::prelude::*;

#[vil_state]
pub struct PipelineState {
    pub request_id: u64,
    pub records_processed: u64,
}

#[vil_event]
pub struct PipelineCompleted {
    pub request_id: u64,
    pub total_records: u64,
    pub duration_us: u64,
}

#[vil_fault]
pub enum PipelineFault {
    UpstreamTimeout,
    TransformFailed,
}

const SINK_PORT: u16 = 3090;
const SINK_PATH: &str = "/trigger";
const SSE_URL: &str = "http://127.0.0.1:4545/v1/chat/completions";

fn configure_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("Gateway")
        .port(SINK_PORT)
        .path(SINK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("LLMUpstream")
        .url(SSE_URL)
        .post_json(serde_json::json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "benchmark multi-pipeline"}],
            "stream": true
        }))
        .json_tap("choices[0].delta.content")
        .done_marker("[DONE]")
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
        .transform(|payload: &[u8]| -> Option<Vec<u8>> {
            let text = String::from_utf8_lossy(payload);
            let enriched = format!("{{\"stage\":\"transform\",\"data\":\"{}\"}}",
                text.replace('"', "\\\"")
            );
            Some(enriched.into_bytes())
        })
}

fn main() {
    // ── Init Runtime (SHM shared) ──────────────────────────────────────
    let world = Arc::new(
        VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime")
    );

    // ── Observer Sidecar ───────────────────────────────────────────────
    vil_observer::sidecar(3190).attach(&world).spawn();

    // ── Configure & Wire Pipeline ──────────────────────────────────────
    let sink_builder = configure_sink();
    let source_builder = configure_source();

    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "MultiPipelineBench",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.response_data_out -> sink_builder.response_data_in (LoanWrite),
            source_builder.response_ctrl_out -> sink_builder.response_ctrl_in (Copy),
        ]
    };

    // ── Banner ─────────────────────────────────────────────────────────
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  101b: Multi-Pipeline Benchmark (ShmToken)                 ║");
    println!("║  3-node: Gateway → Transform → LLM Upstream               ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Pipeline:  http://localhost:{}{}", SINK_PORT, SINK_PATH);
    println!("  Upstream:  {}", SSE_URL);
    println!("  Observer:  http://localhost:3190/_vil/dashboard/");
    println!();

    // ── Build & Run Workers ────────────────────────────────────────────
    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    let sink_thread = sink.run_worker::<ShmToken>(world.clone(), sink_handle);
    let source_thread = source.run_worker::<ShmToken>(world.clone(), source_handle);

    sink_thread.join().expect("Sink panicked");
    source_thread.join().expect("Source panicked");
}
