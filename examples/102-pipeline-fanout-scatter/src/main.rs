// ╔════════════════════════════════════════════════════════════╗
// ║  102 — Loan Portfolio Risk Segmentation (Fan-Out)         ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Banking — Credit Risk Segmentation              ║
// ║  Pattern:  MULTI_PIPELINE (Fan-Out Scatter)               ║
// ║  Token:    ShmToken (shared ExchangeHeap)                 ║
// ║  Nodes:    4 (2 pipelines x 2 nodes each)                 ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Segments loan portfolio into risk buckets by    ║
// ║  scattering credit records across parallel pipelines:      ║
// ║    Pipeline A (NPL):     kol >= 3 (substandard/doubtful/loss)║
// ║    Pipeline B (Healthy): kol < 3 (current/special mention)║
// ║  NPL pipeline feeds provisioning; healthy feeds marketing.║
// ║  Both share SHM for zero-copy concurrent processing.      ║
// ╚════════════════════════════════════════════════════════════╝
//
// Demonstrates the Fan-Out Scatter pattern: a single data source
// (Core Banking NDJSON) is consumed by TWO independent pipelines,
// each applying a different filter transform:
//
//   Pipeline A (NPL):     keeps only records with kolektabilitas >= 3
//   Pipeline B (Healthy): keeps only records with kolektabilitas < 3
//
// Both pipelines share the same VastarRuntimeWorld (ExchangeHeap),
// demonstrating ShmToken's advantage: concurrent sessions on a
// shared memory pool with zero-copy semantics.
//
// Run:
//   cargo run -p fintec01-simulators
//   cargo run -p 102-pipeline-fanout-scatter
//
// Test:
//   # NPL stream (kolektabilitas >= 3)
//   curl -N -X POST http://localhost:3091/npl \
//     -H "Content-Type: application/json" -d '{"request":"npl"}'
//
//   # Healthy stream (kolektabilitas < 3)
//   curl -N -X POST http://localhost:3092/healthy \
//     -H "Content-Type: application/json" -d '{"request":"healthy"}'

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Semantic Types ──────────────────────────────────────────────────────

/// State for the fan-out scatter across both pipelines.
#[vil_state]
pub struct ScatterState {
    pub request_id: u64,
    pub npl_records: u64,
    pub healthy_records: u64,
    pub active_pipelines: u8,
}

/// Emitted when either scatter pipeline completes a batch.
#[vil_event]
pub struct ScatterBatchCompleted {
    pub pipeline_name: u8,
    pub record_count: u64,
    pub filtered_count: u64,
    pub latency_us: u64,
}

/// Faults for the fan-out scatter topology.
#[vil_fault]
pub enum ScatterFault {
    UpstreamTimeout,
    FilterParseError,
    PipelineADisconnect,
    PipelineBDisconnect,
    ShmHeapExhausted,
}

// ── Configuration ───────────────────────────────────────────────────────

const NPL_SINK_PORT: u16 = 3091;
const NPL_SINK_PATH: &str = "/npl";
const HEALTHY_SINK_PORT: u16 = 3092;
const HEALTHY_SINK_PATH: &str = "/healthy";
/// Core Banking NDJSON endpoint — same source data consumed by both risk pipelines.
const NDJSON_URL: &str = "http://localhost:18081/api/v1/credits/ndjson?count=100";

// ── Pipeline A: NPL (Non-Performing Loans, kol >= 3) ───────────────────

fn configure_npl_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("NplSink")
        .port(NPL_SINK_PORT)
        .path(NPL_SINK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_npl_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("NplSource")
        .url(NDJSON_URL)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let record: serde_json::Value = serde_json::from_slice(line).ok()?;
            let kol = record["kolektabilitas"].as_u64().unwrap_or(0);
            // Filter: only keep NPL records (kolektabilitas >= 3)
            if kol >= 3 {
                let mut r = record;
                r["_pipeline"] = serde_json::json!("NPL");
                r["_npl_class"] = serde_json::json!(match kol {
                    3 => "KURANG_LANCAR",
                    4 => "DIRAGUKAN",
                    5 => "MACET",
                    _ => "NPL_OTHER",
                });
                Some(serde_json::to_vec(&r).unwrap_or_else(|_| line.to_vec()))
            } else {
                // Drop healthy records — return None to skip
                None
            }
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Pipeline B: Healthy (Performing Loans, kol < 3) ────────────────────

fn configure_healthy_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("HealthySink")
        .port(HEALTHY_SINK_PORT)
        .path(HEALTHY_SINK_PATH)
        .out_port("trigger_out")
        .in_port("response_data_in")
        .ctrl_in_port("response_ctrl_in")
}

fn configure_healthy_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("HealthySource")
        .url(NDJSON_URL)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            let record: serde_json::Value = serde_json::from_slice(line).ok()?;
            let kol = record["kolektabilitas"].as_u64().unwrap_or(0);
            // Filter: only keep healthy records (kolektabilitas < 3)
            if kol < 3 {
                let mut r = record;
                r["_pipeline"] = serde_json::json!("HEALTHY");
                r["_performing_class"] = serde_json::json!(match kol {
                    1 => "LANCAR",
                    2 => "DALAM_PERHATIAN_KHUSUS",
                    _ => "UNKNOWN",
                });
                Some(serde_json::to_vec(&r).unwrap_or_else(|_| line.to_vec()))
            } else {
                // Drop NPL records — return None to skip
                None
            }
        })
        .in_port("trigger_in")
        .out_port("response_data_out")
        .ctrl_out_port("response_ctrl_out")
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() {
    // Shared ExchangeHeap — both pipelines use the SAME world
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    // ── Pipeline A: NPL ─────────────────────────────────────────────────
    let npl_sink = configure_npl_sink();
    let npl_source = configure_npl_source();

    let (_ir_a, (npl_sink_h, npl_source_h)) = vil_workflow! {
        name: "NplPipeline",
        instances: [ npl_sink, npl_source ],
        routes: [
            npl_sink.trigger_out -> npl_source.trigger_in (LoanWrite),
            npl_source.response_data_out -> npl_sink.response_data_in (LoanWrite),
            npl_source.response_ctrl_out -> npl_sink.response_ctrl_in (Copy),
        ]
    };

    // ── Pipeline B: Healthy ─────────────────────────────────────────────
    let healthy_sink = configure_healthy_sink();
    let healthy_source = configure_healthy_source();

    let (_ir_b, (healthy_sink_h, healthy_source_h)) = vil_workflow! {
        name: "HealthyPipeline",
        instances: [ healthy_sink, healthy_source ],
        routes: [
            healthy_sink.trigger_out -> healthy_source.trigger_in (LoanWrite),
            healthy_source.response_data_out -> healthy_sink.response_data_in (LoanWrite),
            healthy_source.response_ctrl_out -> healthy_sink.response_ctrl_in (Copy),
        ]
    };

    // Banner
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  102 — Fan-Out Scatter (ShmToken, Multi-Pipeline)        ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║                                                            ║");
    println!("║  Pipeline A (NPL):                                         ║");
    println!("║    Sink(:3091/npl) ──> Source(NDJSON, filter kol>=3)      ║");
    println!("║                                                            ║");
    println!("║  Pipeline B (Healthy):                                     ║");
    println!("║    Sink(:3092/healthy) ──> Source(NDJSON, filter kol<3)   ║");
    println!("║                                                            ║");
    println!("║  Shared: ExchangeHeap (ShmToken, zero-copy)                ║");
    println!("║  Source: Core Banking Simulator (:18081)                    ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Requires: Core Banking Simulator on port 18081");
    println!("    cargo run -p fintec01-simulators");
    println!();
    println!("  Test NPL stream (kolektabilitas >= 3):");
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        NPL_SINK_PORT, NPL_SINK_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"npl\"}}'");
    println!();
    println!("  Test Healthy stream (kolektabilitas < 3):");
    println!(
        "  curl -N -X POST http://localhost:{}{} \\",
        HEALTHY_SINK_PORT, HEALTHY_SINK_PATH
    );
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"request\":\"healthy\"}}'");
    println!();

    // Build nodes — 4 workers for 2 parallel risk segmentation pipelines.
    // NPL pipeline feeds provisioning calculations and risk management.
    // Healthy pipeline feeds cross-sell/upsell marketing campaigns.
    let npl_sink_node = HttpSink::from_builder(npl_sink);
    let npl_source_node = HttpSource::from_builder(npl_source);
    let healthy_sink_node = HttpSink::from_builder(healthy_sink);
    let healthy_source_node = HttpSource::from_builder(healthy_source);

    // All 4 workers share the SAME ExchangeHeap — ShmToken enables
    // concurrent zero-copy sessions across both risk segmentation pipelines.
    let t1 = npl_sink_node.run_worker::<ShmToken>(world.clone(), npl_sink_h);
    let t2 = npl_source_node.run_worker::<ShmToken>(world.clone(), npl_source_h);
    let t3 = healthy_sink_node.run_worker::<ShmToken>(world.clone(), healthy_sink_h);
    let t4 = healthy_source_node.run_worker::<ShmToken>(world.clone(), healthy_source_h);

    t1.join().expect("NplSink worker panicked");
    t2.join().expect("NplSource worker panicked");
    t3.join().expect("HealthySink worker panicked");
    t4.join().expect("HealthySource worker panicked");
}
