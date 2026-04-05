// ╔════════════════════════════════════════════════════════════════════════╗
// ║  106 — SSE Standard Dialect Demo (W3C Spec)                         ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  SDK_PIPELINE                                              ║
// ║  Token:    ShmToken                                                  ║
// ║  Features: SseSourceDialect::Standard, done_marker("[END]")          ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: Demonstrates W3C-compliant SSE Standard dialect with      ║
// ║  done_marker support. Upstream is a banking credit data simulator    ║
// ║  (not IoT sensors). The SSE pattern itself is domain-agnostic and   ║
// ║  works with any W3C SSE-compatible source.                           ║
// ║                                                                      ║
// ║  Why Standard SSE dialect:                                           ║
// ║    - W3C spec-compliant: works with any browser's EventSource API   ║
// ║    - No JSON envelope overhead (unlike OpenAI/Anthropic dialects)   ║
// ║    - Each SSE "data:" line is the raw sensor payload                ║
// ║    - Custom done_marker("[END]") signals the batch is complete      ║
// ║                                                                      ║
// ║  Architecture:                                                       ║
// ║    [IoT Sensors] → [SSE Source] → SHM Pipeline → [SSE Sink/HTTP]    ║
// ║                                                                      ║
// ║  Pipeline topology:                                                  ║
// ║    Sink (HTTP endpoint) ←trigger→ Source (SSE upstream)              ║
// ║    Source streams data back to Sink via Data Lane                    ║
// ║    Source sends completion signal via Control Lane                   ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-pipeline-sse-standard-dialect
// Test: curl -N -X POST http://localhost:3106/stream \
//         -H "Content-Type: application/json" \
//         -d '{"factory_id":"F-101","sensor_type":"temperature"}'

use std::sync::Arc;
use vil_sdk::prelude::*;

// ── IoT Semantic Types ──────────────────────────────────────────────────
// These types describe the pipeline's state, events, and failure modes
// using VIL's semantic type system. The runtime uses these for:
// - Observability: dashboards show SensorStreamState in real-time
// - Alerting: SensorBatchCompleted events trigger downstream processing
// - Fault handling: SensorStreamFault drives retry/failover decisions

/// Pipeline state: tracks how many sensor readings have been received.
/// The IoT dashboard polls this to show "X readings collected so far."
#[vil_state]
pub struct SensorStreamState {
    pub factory_id: u64,
    pub readings_received: u64,
    pub batches_completed: u64,
}

/// Event emitted when a sensor batch finishes streaming.
/// Downstream services (analytics, anomaly detection) subscribe to this
/// to know when a complete batch is ready for processing.
#[vil_event]
pub struct SensorBatchCompleted {
    pub factory_id: u64,
    pub total_readings: u64,
    pub done_marker_seen: bool,
}

/// Fault types for the IoT sensor pipeline.
/// Each variant maps to a real-world IoT failure scenario.
#[vil_fault]
pub enum SensorStreamFault {
    /// Sensor gateway did not respond within timeout (network/power issue)
    SensorGatewayTimeout,
    /// SSE stream contained malformed data (corrupt sensor firmware)
    InvalidSensorFormat,
    /// Stream ended without the expected "[END]" marker (incomplete batch)
    BatchIncomplete,
}

// ── Pipeline Configuration ──────────────────────────────────────────────
// In a real IoT deployment, these would come from environment variables
// or a configuration service. The sink exposes an HTTP endpoint for
// the dashboard, and the source connects to the IoT gateway's SSE stream.

const DASHBOARD_PORT: u16 = 3106;
const DASHBOARD_PATH: &str = "/stream";
const IOT_GATEWAY_URL: &str = "http://localhost:18081/api/v1/credits/stream";

/// Configure the HTTP sink — this is the endpoint the IoT dashboard connects to.
/// The dashboard sends a POST to trigger data collection, then receives
/// sensor readings streamed back via SSE.
fn configure_dashboard_sink() -> HttpSinkBuilder {
    HttpSinkBuilder::new("IoTDashboardSink")
        .port(DASHBOARD_PORT)
        .path(DASHBOARD_PATH)
        .out_port("trigger_out") // "start collecting sensor data"
        .in_port("sensor_data_in") // receives sensor readings
        .ctrl_in_port("batch_ctrl_in") // receives batch completion signal
}

/// Configure the SSE source — connects to the IoT gateway's sensor stream.
/// Uses Standard dialect (W3C SSE spec) because IoT gateways typically
/// output plain text sensor data, not JSON-enveloped AI responses.
fn configure_sensor_source() -> HttpSourceBuilder {
    HttpSourceBuilder::new("IoTSensorSource")
        .url(IOT_GATEWAY_URL)
        .format(HttpFormat::SSE)
        // Standard dialect: raw "data:" lines, no JSON envelope.
        // Each line contains a sensor reading like: "temp=22.5|humidity=45|pressure=1013"
        .dialect(SseSourceDialect::Standard)
        // Custom done marker: when the IoT gateway sends "[END]", the batch is complete.
        // This is important because sensor batches have a fixed duration (e.g., 1 minute).
        .done_marker("[END]")
        .in_port("trigger_in") // triggered by dashboard request
        .out_port("sensor_data_out") // streams sensor readings
        .ctrl_out_port("batch_ctrl_out") // signals batch completion
}

// ── Main ────────────────────────────────────────────────────────────────

fn main() {
    // Initialize VIL's shared memory runtime.
    // SHM allows sensor data to flow between pipeline nodes at memory speed.
    let world = Arc::new(VastarRuntimeWorld::new_shared().expect("Failed to init VIL SHM Runtime"));

    let sink = configure_dashboard_sink();
    let source = configure_sensor_source();

    // Build the pipeline using vil_workflow! macro.
    // Routes define how data flows between nodes:
    //   - Sink triggers Source to start collecting sensor data (LoanWrite)
    //   - Source streams sensor readings back to Sink (LoanWrite = zero-copy SHM)
    //   - Source sends batch completion via Control Lane (Copy = small signal)
    let (_ir, (sink_h, source_h)) = vil_workflow! {
        name: "IoTSensorPipeline",
        instances: [ sink, source ],
        routes: [
            sink.trigger_out -> source.trigger_in (LoanWrite),
            source.sensor_data_out -> sink.sensor_data_in (LoanWrite),
            source.batch_ctrl_out -> sink.batch_ctrl_in (Copy),
        ]
    };

    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  106 — SSE Standard Dialect Demo (W3C Spec)                          ║");
    println!("╠════════════════════════════════════════════════════════════════════════╣");
    println!("║  Dialect: SseSourceDialect::Standard (W3C SSE — no JSON envelope)    ║");
    println!("║  Done:    [END] marker signals batch completion                      ║");
    println!("║  Data:    Sensor readings flow via zero-copy SHM pipeline            ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!(
        "  Dashboard:   http://localhost:{}{}",
        DASHBOARD_PORT, DASHBOARD_PATH
    );
    println!("  IoT Gateway: {}", IOT_GATEWAY_URL);
    println!();

    // Create pipeline nodes and run on dedicated worker threads.
    // Each node runs its own event loop — no shared async executor.
    let sink_node = HttpSink::from_builder(sink);
    let source_node = HttpSource::from_builder(source);

    let t1 = sink_node.run_worker::<ShmToken>(world.clone(), sink_h);
    let t2 = source_node.run_worker::<ShmToken>(world.clone(), source_h);

    t1.join().expect("IoT Dashboard Sink panicked");
    t2.join().expect("IoT Sensor Source panicked");
}
