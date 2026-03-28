// ╔════════════════════════════════════════════════════════════╗
// ║  101 — ETL Data Pipeline (Extract-Transform-Load)         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Data Engineering — Credit Portfolio ETL          ║
// ║  Pattern:  SDK_PIPELINE                                   ║
// ║  Token:    GenericToken                                    ║
// ║  Nodes:    2 (Sink + Source with chained transforms)       ║
// ║  Topology: HttpSink(:3090) -> HttpSource(NDJSON :18081)   ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Classic ETL pipeline for credit data:           ║
// ║    Extract  — stream NDJSON from Core Banking              ║
// ║    Transform — 3-step chain:                               ║
// ║      Step 1: Normalize (uppercase borrower names)          ║
// ║      Step 2: Enrich (compute risk score from kol + saldo) ║
// ║      Step 3: Classify (assign HIGH/MEDIUM/LOW risk class) ║
// ║    Load — stream enriched records to downstream consumer  ║
// ╚════════════════════════════════════════════════════════════╝
//
// Demonstrates chaining multiple transform operations within a single
// HttpSource node. Three logical steps in one closure:
//   Step 1: Normalize — uppercase nama_lengkap
//   Step 2: Enrich   — compute _risk_score from kolektabilitas + saldo
//   Step 3: Classify — assign _risk_class (HIGH / MEDIUM / LOW)
//
// Uses GenericToken (sample-table based routing) since this is a
// single-pipeline scenario with no shared-memory requirements.
//
// Run:
//   cargo run -p fintec01-simulators   # start Core Banking Simulator
//   cargo run -p 101-pipeline-3node-transform-chain
//
// Test:
//   curl -N -X POST http://localhost:3090/transform \
//     -H "Content-Type: application/json" \
//     -d '{"request":"chain-transforms"}'

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Semantic Types ──────────────────────────────────────────────────────

/// State tracking for the 3-step transform chain pipeline.
#[vil_state]
pub struct TransformChainState {
    pub request_id: u64,
    pub records_processed: u64,
    pub high_risk_count: u32,
    pub medium_risk_count: u32,
    pub low_risk_count: u32,
}

/// Emitted when a transform chain batch completes.
#[vil_event]
pub struct TransformChainCompleted {
    pub request_id: u64,
    pub total_records: u64,
    pub high_risk_count: u32,
    pub duration_us: u64,
}

/// Faults specific to the transform chain pipeline.
#[vil_fault]
pub enum TransformChainFault {
    UpstreamTimeout,
    JsonParseError,
    TransformFailed,
    SinkWriteError,
}

// ── Configuration ───────────────────────────────────────────────────────

const SINK_PORT: u16 = 3090;
const SINK_PATH: &str = "/transform";
const NDJSON_URL: &str = "http://localhost:18081/api/v1/credits/ndjson?count=100";

// ── Node Builders ───────────────────────────────────────────────────────

fn configure_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("TransformGateway")
        .port(SINK_PORT)
        .path(SINK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("ChainedTransformSource")
        .url(NDJSON_URL)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let mut r: serde_json::Value = serde_json::from_slice(line).ok()?;

            // Step 1: Normalize — uppercase nama_lengkap
            if let Some(nama) = r["nama_lengkap"].as_str() {
                r["nama_lengkap"] = serde_json::json!(nama.to_uppercase());
            }

            // Step 2: Enrich — compute risk score
            //   risk_score = kolektabilitas * 20 + saldo / 1_000_000
            let kol = r["kolektabilitas"].as_u64().unwrap_or(0);
            let saldo = r["saldo_outstanding"].as_f64().unwrap_or(0.0);
            let risk_score = kol as f64 * 20.0 + saldo / 1_000_000.0;
            r["_risk_score"] = serde_json::json!((risk_score * 100.0).round() / 100.0);

            // Step 3: Classify — assign risk class based on score thresholds
            r["_risk_class"] = serde_json::json!(if risk_score > 100.0 {
                "HIGH"
            } else if risk_score > 50.0 {
                "MEDIUM"
            } else {
                "LOW"
            });

            Some(serde_json::to_vec(&r).unwrap_or_else(|_| line.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() {
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    let sink_builder = configure_sink();
    let source_builder = configure_source();

    // Wire pipeline: sink <-> source with Tri-Lane routing
    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "TransformChainPipeline",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.response_data_out -> sink_builder.response_data_in (LoanWrite),
            source_builder.response_ctrl_out -> sink_builder.response_ctrl_in (Copy),
        ]
    };

    // Banner
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  101 — 3-Node Transform Chain (GenericToken)             ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║                                                            ║");
    println!("║  Gateway (:3090/transform) ──[LoanWrite]──> Source       ║");
    println!("║                            <──[LoanWrite]── (NDJSON)     ║");
    println!("║                            <──[Copy]─────── (ctrl done)  ║");
    println!("║                                                            ║");
    println!("║  Transform Chain:                                          ║");
    println!("║    Step 1: Normalize (uppercase nama_lengkap)              ║");
    println!("║    Step 2: Enrich   (compute _risk_score)                  ║");
    println!("║    Step 3: Classify (_risk_class: HIGH/MEDIUM/LOW)         ║");
    println!("║                                                            ║");
    println!("║  Token:  GenericToken (single pipeline, sample-table)      ║");
    println!("║  Format: NDJSON (Core Banking Simulator :18081)            ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Requires: Core Banking Simulator on port 18081");
    println!("    cargo run -p fintec01-simulators");
    println!();
    println!("  Test:");
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        SINK_PORT, SINK_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"chain-transforms\"}}'");
    println!();
    println!("  Benchmark:");
    println!("  oha -m POST --no-tui -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"bench\"}}' -c 50 -n 500 \\");
    println!("    http://localhost:{}{}", SINK_PORT, SINK_PATH);
    println!();

    // Build nodes and spawn workers — GenericToken for single pipeline
    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    let t1 = sink.run_worker::<GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<GenericToken>(world.clone(), source_handle);

    t1.join().expect("TransformGateway worker panicked");
    t2.join().expect("ChainedTransformSource worker panicked");
}
