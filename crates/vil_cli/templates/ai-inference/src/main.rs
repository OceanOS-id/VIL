//! SSE Inference Gateway — VIL Template
//!
//! Zero-copy HTTP proxy to SSE streaming endpoints.
//! Default upstream: Core Banking Simulator credit stream.
//!
//! ## Prerequisites
//!
//! ```bash
//! # Start Core Banking Simulator:
//! cargo run -p fintec01-simulators
//! ```
//!
//! ## Usage
//!
//! ```bash
//! cargo run --release
//!
//! # Test:
//! curl -N -X POST -H "Content-Type: application/json" \
//!   -d '{"request": "stream-credits"}' http://localhost:3080/trigger
//!
//! # Load test:
//! oha -m POST -H "Content-Type: application/json" \
//!   -d '{"request": "bench"}' -c 50 -n 500 http://localhost:3080/trigger
//! ```

use std::sync::Arc;
use vil_sdk::prelude::*;

// Pipeline Configuration
const WEBHOOK_PORT: u16 = 3080;
const WEBHOOK_PATH: &str = "/trigger";
/// Core Banking Simulator SSE endpoint — streams credit records.
const SSE_URL: &str =
    "http://localhost:18081/api/v1/credits/stream?count=50&batch_size=10&delay_ms=100";

fn main() {
    // Step 1: Init VIL Runtime
    let world = Arc::new(
        VastarRuntimeWorld::new_shared().expect("Failed to initialize VIL SHM Runtime"),
    );

    // Step 2: Configure Nodes
    let sink_builder = HttpSinkBuilder::new("WebhookTrigger")
        .port(WEBHOOK_PORT)
        .path(WEBHOOK_PATH)
        .out_port("trigger_out")
        .in_port("data_in")
        .ctrl_in_port("ctrl_in");

    // Core Banking SSE — Standard dialect, GET method (default)
    let source_builder = HttpSourceBuilder::new("SseIngest")
        .url(SSE_URL)
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::Standard)
        .in_port("trigger_in")
        .out_port("data_out")
        .ctrl_out_port("ctrl_out");

    // Step 3: Wire Pipeline
    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "SseIngestGateway",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.data_out -> sink_builder.data_in (LoanWrite),
            source_builder.ctrl_out -> sink_builder.ctrl_in (Copy),
        ]
    };

    // Step 4: Banner
    println!("╔══════════════════════════════════════════════════╗");
    println!("║  VIL SSE Ingest Gateway                      ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!(
        "  Webhook:  http://localhost:{}{}",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!("  Upstream: {}", SSE_URL);
    println!();
    println!("  curl -N -X POST -H 'Content-Type: application/json' \\");
    println!(
        "    -d '{{\"request\": \"test\"}}' http://localhost:{}{}",
        WEBHOOK_PORT, WEBHOOK_PATH
    );
    println!();

    // Step 5: Spawn Workers
    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    let t1 = sink.run_worker::<GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<GenericToken>(world.clone(), source_handle);

    t1.join().expect("Sink worker panicked");
    t2.join().expect("Source worker panicked");
}
