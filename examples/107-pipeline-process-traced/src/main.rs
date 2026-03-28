// ╔════════════════════════════════════════════════════════════════════════╗
// ║  107 — Supply Chain Tracking (Pipeline Process Traced)              ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  SDK_PIPELINE                                              ║
// ║  Token:    ShmToken                                                  ║
// ║  Features: #[process], #[trace_hop], #[latency_marker]               ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: A logistics company tracks packages through the supply    ║
// ║  chain. Each stage (warehouse scan, shipping carrier handoff,        ║
// ║  delivery confirmation) is a traced "hop" in the pipeline.           ║
// ║                                                                      ║
// ║  Why #[process] + #[trace_hop] matter:                               ║
// ║    - End-to-end latency measurement across the entire supply chain  ║
// ║    - Each hop records its processing time automatically              ║
// ║    - Operations team can see exactly WHERE delays occur:             ║
// ║      "Warehouse scan took 50ms, but carrier handoff took 3 seconds" ║
// ║    - No manual timing code — VIL instruments at compile time        ║
// ║                                                                      ║
// ║  Supply chain stages:                                                ║
// ║    [Warehouse Scan] → [Carrier Handoff] → [Delivery Confirmation]   ║
// ║         hop #1              hop #2               hop #3              ║
// ║      latency: 50ms      latency: 200ms       latency: 100ms        ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-pipeline-process-traced
// Test: curl -N -X POST http://localhost:3107/traced \
//         -H "Content-Type: application/json" \
//         -d '{"tracking_id":"PKG-2026-0042","origin":"Seattle","destination":"New York"}'

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── Supply Chain Semantic Types ─────────────────────────────────────────

/// Pipeline state: tracks how many hops a package has passed through
/// and the cumulative latency across the supply chain.
#[vil_state]
pub struct SupplyChainState {
    pub tracking_id: u64,
    pub hops_completed: u64,
    pub total_latency_us: u64,
}

/// Event emitted when a package completes one hop in the supply chain.
/// The operations dashboard subscribes to these events to build a
/// real-time tracking timeline for each package.
#[vil_event]
pub struct PackageHopCompleted {
    pub hop_name: u64,
    pub latency_us: u64,
    pub process_id: u64,
}

/// Fault types for supply chain tracking.
#[vil_fault]
pub enum SupplyChainFault {
    /// A stage took longer than its SLA (e.g., warehouse scan > 5 minutes)
    StageTimeout,
    /// Barcode/QR scan failed at a handoff point
    ScanFailed,
    /// Trace data is corrupted (missing hop, out-of-order timestamps)
    TraceCorrupted,
}

// ── Traced Supply Chain Processes ────────────────────────────────────────
//
// #[process(trace_hop, latency = "...")] does two things at compile time:
//   1. Generates an IR builder for this process (visible in pipeline IR)
//   2. Instruments the process with automatic latency recording
//
// When this process runs, VIL records:
//   - Entry timestamp (when the package arrives at this stage)
//   - Exit timestamp (when processing completes)
//   - Delta = latency for this hop (written to observability pipeline)

/// Warehouse Scan: the first hop in the supply chain.
/// Workers scan the package barcode, weigh it, and record dimensions.
/// Typical latency: 30-100ms (barcode scan + database write).
#[process(trace_hop, latency = "warehouse_scan_latency")]
struct WarehouseScanProcess;

/// Carrier Handoff: the package is loaded onto a shipping truck.
/// The carrier's API is called to generate a tracking label.
/// Typical latency: 100-500ms (external API call to carrier).
#[process(trace_hop, latency = "carrier_handoff_latency")]
struct CarrierHandoffProcess;

// ── Pipeline Configuration ──────────────────────────────────────────────

const TRACKING_PORT: u16 = 3107;
const TRACKING_PATH: &str = "/traced";
const UPSTREAM_URL: &str = "http://localhost:18081/api/v1/credits/stream";

/// Configure the tracking API sink — the endpoint where tracking queries arrive.
fn configure_tracking_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("TrackingSink")
        .port(TRACKING_PORT)
        .path(TRACKING_PATH)
        .out_port("trigger_out")
        .in_port("tracking_data_in")
        .ctrl_in_port("delivery_ctrl_in")
}

/// Configure the upstream source — connects to the warehouse/carrier systems.
/// The transform enriches each tracking event with trace metadata so the
/// operations team can see which hop produced each data point.
fn configure_upstream_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("SupplyChainSource")
        .url(UPSTREAM_URL)
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::Standard)
        .done_marker("[DONE]")
        .transform(|data: &[u8]| {
            // Enrich each tracking event with supply chain trace metadata.
            // In production, this would add: hop_name, warehouse_id, carrier_code,
            // GPS coordinates, weight, and the measured latency for this hop.
            let mut record: serde_json::Value = serde_json::from_slice(data).ok()?;
            record["_traced"] = serde_json::json!(true);
            record["_hop"] = serde_json::json!("carrier_handoff");
            record["_supply_chain"] = serde_json::json!("PKG pipeline v2");
            Some(serde_json::to_vec(&record).unwrap_or_else(|_| data.to_vec()))
        })
        .in_port("trigger_in")
        .out_port("tracking_data_out")
        .ctrl_out_port("delivery_ctrl_out")
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() {
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    let sink = configure_tracking_sink();
    let source = configure_upstream_source();

    let (_ir, (sink_h, source_h)) = vil_workflow! {
        name: "SupplyChainTrackedPipeline",
        instances: [ sink, source ],
        routes: [
            sink.trigger_out -> source.trigger_in (LoanWrite),
            source.tracking_data_out -> sink.tracking_data_in (LoanWrite),
            source.delivery_ctrl_out -> sink.delivery_ctrl_in (Copy),
        ]
    };

    // Verify that #[process] generated IR builders for both traced stages.
    // These builders are used by the VIL compiler to emit observability hooks.
    let _warehouse_ir = WarehouseScanProcess::get_process_builder();
    let _carrier_ir = CarrierHandoffProcess::get_process_builder();

    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  107 — Supply Chain Tracking (Pipeline Process Traced)               ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  Hop 1: Warehouse Scan     (trace_hop + warehouse_scan_latency)      ║");
    println!("║  Hop 2: Carrier Handoff    (trace_hop + carrier_handoff_latency)     ║");
    println!("║  Each hop auto-records entry/exit timestamps for latency tracking    ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!(
        "  Tracking API: http://localhost:{}{}",
        TRACKING_PORT, TRACKING_PATH
    );
    println!("  Upstream:     {}", UPSTREAM_URL);

    let sink_node = HttpSink::from_builder(sink);
    let source_node = HttpSource::from_builder(source);

    let t1 = sink_node.run_worker::<ShmToken>(world.clone(), sink_h);
    let t2 = source_node.run_worker::<ShmToken>(world.clone(), source_h);

    t1.join().expect("Tracking Sink panicked");
    t2.join().expect("Supply Chain Source panicked");
}
