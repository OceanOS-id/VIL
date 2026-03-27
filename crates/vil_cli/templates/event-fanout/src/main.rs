//! Event Fanout — VIL Template
//!
//! One-to-many event broadcast via zero-copy shared memory.
//! Default consumer: Core Banking Simulator SSE credit stream.
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
//!   -d '{"event": "credit-sync"}' http://localhost:8080/event
//! ```

use std::sync::Arc;
use vil_sdk::prelude::*;

const LISTEN_PORT: u16 = 8080;
const LISTEN_PATH: &str = "/event";
/// Core Banking Simulator SSE endpoint.
const CONSUMER_URL: &str =
    "http://localhost:18081/api/v1/credits/stream?count=50&batch_size=10&delay_ms=100";

fn main() {
    let world = Arc::new(
        VastarRuntimeWorld::new_shared().expect("Failed to initialize VIL SHM Runtime"),
    );

    let sink_builder = HttpSinkBuilder::new("EventIngress")
        .port(LISTEN_PORT)
        .path(LISTEN_PATH)
        .out_port("trigger_out")
        .in_port("data_in")
        .ctrl_in_port("ctrl_in");

    // Core Banking SSE — Standard dialect, GET method (default)
    let source_builder = HttpSourceBuilder::new("Consumer")
        .url(CONSUMER_URL)
        .format(HttpFormat::SSE)
        .dialect(SseSourceDialect::Standard)
        .in_port("trigger_in")
        .out_port("data_out")
        .ctrl_out_port("ctrl_out");

    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "EventFanout",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.data_out -> sink_builder.data_in (LoanWrite),
            source_builder.ctrl_out -> sink_builder.ctrl_in (Copy),
        ]
    };

    println!("VIL Event Fanout");
    println!(
        "  Listen:   http://localhost:{}{}",
        LISTEN_PORT, LISTEN_PATH
    );
    println!("  Consumer: {}", CONSUMER_URL);
    println!();

    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    let t1 = sink.run_worker::<GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<GenericToken>(world.clone(), source_handle);

    t1.join().expect("Sink worker panicked");
    t2.join().expect("Source worker panicked");
}
