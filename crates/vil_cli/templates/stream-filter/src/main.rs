//! Stream Filter — VIL Template
//!
//! Filter and transform streaming data with zero-copy semantics.
//! Connects to Core Banking Simulator SSE for credit record streaming.
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
//! curl -N -X POST -H "Content-Type: application/json" \
//!   -d '{"filter": "npl"}' http://localhost:8080/stream
//! ```

use std::sync::Arc;
use vil_sdk::prelude::*;

const LISTEN_PORT: u16 = 8080;
const LISTEN_PATH: &str = "/stream";
/// Core Banking Simulator SSE endpoint — streams credit records in batches.
const UPSTREAM_URL: &str =
    "http://localhost:18081/api/v1/credits/stream?count=100&batch_size=10&delay_ms=50";

fn main() {
    let world = Arc::new(
        VastarRuntimeWorld::new_shared().expect("Failed to initialize VIL SHM Runtime"),
    );

    let sink_builder = HttpSinkBuilder::new("StreamIngress")
        .port(LISTEN_PORT)
        .path(LISTEN_PATH)
        .out_port("trigger_out")
        .in_port("data_in")
        .ctrl_in_port("ctrl_in");

    // Core Banking SSE — Standard dialect, GET method (default)
    let source_builder = HttpSourceBuilder::new("StreamUpstream")
        .url(UPSTREAM_URL)
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::Standard)
        .in_port("trigger_in")
        .out_port("data_out")
        .ctrl_out_port("ctrl_out");

    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "StreamFilter",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.data_out -> sink_builder.data_in (LoanWrite),
            source_builder.ctrl_out -> sink_builder.ctrl_in (Copy),
        ]
    };

    println!("VIL Stream Filter");
    println!(
        "  Listen:   http://localhost:{}{}",
        LISTEN_PORT, LISTEN_PATH
    );
    println!("  Upstream: {}", UPSTREAM_URL);
    println!();

    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    let t1 = sink.run_worker::<GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<GenericToken>(world.clone(), source_handle);

    t1.join().expect("Sink worker panicked");
    t2.join().expect("Source worker panicked");
}
