// ╔════════════════════════════════════════════════════════════╗
// ║  007 — Non-Performing Loan Detection System               ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   Banking — Credit Risk Management                ║
// ║  Pattern:  SDK_PIPELINE                                     ║
// ║  Token:    ShmToken (zero-copy for bulk credit streaming)  ║
// ║  Features: .transform(), vil_workflow!, #[vil_fault]        ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Streams credit records from Core Banking and    ║
// ║  filters for Non-Performing Loans (NPL). Per OJK regs:    ║
// ║    kol 3 = Kurang Lancar (Substandard)                     ║
// ║    kol 4 = Diragukan (Doubtful)                            ║
// ║    kol 5 = Macet (Loss)                                    ║
// ║  NPL records trigger provisioning alerts and feed into     ║
// ║  the monthly SLIK regulatory report (see example 009).     ║
// ╚════════════════════════════════════════════════════════════╝
// Run:
//   # Start Core Banking Simulator first:
//   cargo run -p fintec01-simulators
//
//   # Then run this example:
//   cargo run -p basic-usage-credit-npl-filter
//
// Test:
//   curl -N -X POST -H "Content-Type: application/json" \
//     -d '{"filter": "npl"}' \
//     http://localhost:3081/filter-npl

use std::sync::Arc;
use vil_sdk::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// VIL Semantic Types — Credit NPL detection domain
// ─────────────────────────────────────────────────────────────────────────────

/// Accumulated state of the NPL detection stream.
#[vil_state]
pub struct NplFilterState {
    pub request_id: u64,
    pub total_records: u32,
    pub npl_detected: u32,
    pub total_npl_exposure: u64,
    pub stream_active: bool,
}

/// Emitted when an NPL credit is detected in the stream.
#[vil_event]
pub struct NplDetected {
    pub request_id: u64,
    pub credit_id: u64,
    pub kolektabilitas: u8,
    pub saldo_outstanding: u64,
    pub timestamp_us: u64,
}

/// Fault domain for the NPL detection pipeline.
#[vil_fault]
pub enum NplFilterFault {
    CoreBankingTimeout,
    StreamDisconnect,
    InvalidCreditRecord,
    ShmWriteFailed,
}

// ─────────────────────────────────────────────────────────────────────────────
// PIPELINE CONFIGURATION
// ─────────────────────────────────────────────────────────────────────────────

const WEBHOOK_PORT: u16 = 3081;
const WEBHOOK_PATH: &str = "/filter-npl";

/// Core Banking NDJSON endpoint with higher dirty_ratio to simulate
/// more problematic credit data (30% dirty records).
const CORE_BANKING_NDJSON: &str =
    "http://localhost:18081/api/v1/credits/ndjson?count=1000&dirty_ratio=0.3";

const P_TRIGGER_OUT: &str = "trigger_out";
const P_TRIGGER_IN: &str = "trigger_in";
const P_DATA_IN: &str = "response_data_in";
const P_DATA_OUT: &str = "response_data_out";
const P_CTRL_IN: &str = "response_ctrl_in";
const P_CTRL_OUT: &str = "response_ctrl_out";

// ─────────────────────────────────────────────────────────────────────────────
// NODE CONFIGURATION (Decomposed Builder Style)
// ─────────────────────────────────────────────────────────────────────────────

fn configure_webhook_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("NplFilterSink")
        .port(WEBHOOK_PORT)
        .path(WEBHOOK_PATH)
        .out_port(P_TRIGGER_OUT)
        .in_port(P_DATA_IN)
        .ctrl_in_port(P_CTRL_IN)
}

fn configure_credit_source() -> HttpSourceBuilder {
    // Core Banking NDJSON — one JSON record per line
    // Transform: filter only NPL credits (kolektabilitas >= 3)
    HttpSourceBuilder::new("NplCreditSource")
        .url(CORE_BANKING_NDJSON)
        .format(HttpFormat::NDJSON)
        .transform(|line: &[u8]| {
            // Parse each NDJSON line and filter for NPL
            let record: serde_json::Value = serde_json::from_slice(line).ok()?;
            let kol = record["kolektabilitas"].as_u64().unwrap_or(0);
            if kol >= 3 {
                // NPL credit (kolektabilitas 3=Kurang Lancar, 4=Diragukan, 5=Macet)
                Some(line.to_vec())
            } else {
                None // Healthy credit — filtered out
            }
        })
        .in_port(P_TRIGGER_IN)
        .out_port(P_DATA_OUT)
        .ctrl_out_port(P_CTRL_OUT)
}

fn main() {
    // ── Step 1: Init Runtime ─────────────────────────────────────────────────
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    // ── Step 2: Configure Nodes (Decomposed Style) ──────────────────────────
    let sink_builder = configure_webhook_sink();
    let source_builder = configure_credit_source();

    // ── Step 3: Wire Pipeline via vil_workflow! Macro ──────────────────────
    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "NplFilterPipeline",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.response_data_out -> sink_builder.response_data_in (LoanWrite),
            source_builder.response_ctrl_out -> sink_builder.response_ctrl_in (Copy),
        ]
    };

    // ── Step 4: Startup Banner ───────────────────────────────────────────────
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  006 — Credit NPL Stream Filter Pipeline                       ║");
    println!("║  Layer 3 — vil_workflow! (Decomposed Builder)                ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║                                                                  ║");
    println!("║  PIPELINE FLOW:                                                  ║");
    println!("║                                                                  ║");
    println!("║    POST /filter-npl                                              ║");
    println!("║         |                                                        ║");
    println!("║         v                                                        ║");
    println!("║    [WebhookSink] --trigger--> [Core Banking SSE Source]          ║");
    println!("║         ^                          |                             ║");
    println!("║         |                          v                             ║");
    println!("║         +--- data + ctrl --- credit records (NDJSON lines)        ║");
    println!("║                                                                  ║");
    println!("║  BUSINESS LOGIC:                                                 ║");
    println!("║    Streams credit records from Core Banking Simulator.           ║");
    println!("║    In production: filters for kolektabilitas >= 3 (NPL).         ║");
    println!("║    NPL = Kurang Lancar / Diragukan / Macet per OJK rules.       ║");
    println!("║                                                                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();
    println!(
        "  Listening on http://localhost:{}{}",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!("  Upstream NDJSON: {}", CORE_BANKING_NDJSON);
    println!("  Format:      NDJSON");
    println!("  Token:        ShmToken (zero-copy)");
    println!();
    println!("  Requires: Core Banking Simulator running on port 18081");
    println!("    cargo run -p fintec01-simulators");
    println!();
    println!("  Test with:");
    println!("  curl -N -X POST -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"filter\": \"npl\"}}' \\");
    println!("    http://localhost:{}{}", WEBHOOK_PORT, WEBHOOK_PATH);
    println!();
    println!("  Load test (oha):");
    println!("  oha -m POST -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"filter\": \"npl\"}}' -c 50 -n 500 \\");
    println!("    http://localhost:{}{}", WEBHOOK_PORT, WEBHOOK_PATH);
    println!();

    // ── Step 5: Build & Spawn Workers ────────────────────────────────────────
    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    let sink_thread = sink.run_worker::<ShmToken>(world.clone(), sink_handle);
    let source_thread = source.run_worker::<ShmToken>(world.clone(), source_handle);

    sink_thread.join().expect("NplFilterSink worker panicked");
    source_thread
        .join()
        .expect("NplCreditSource worker panicked");
}
