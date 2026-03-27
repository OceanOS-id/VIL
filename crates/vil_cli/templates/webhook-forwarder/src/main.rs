//! Webhook Forwarder — VIL Template
//!
//! Receives webhooks and forwards to downstream services via zero-copy IPC.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --release
//!
//! curl -X POST -H "Content-Type: application/json" \
//!   -d '{"event": "push", "repo": "my-app"}' http://localhost:8080/webhook
//! ```

use std::sync::Arc;
use vil_sdk::prelude::*;

const LISTEN_PORT: u16 = 8080;
const LISTEN_PATH: &str = "/webhook";
const DOWNSTREAM_URL: &str = "http://127.0.0.1:9000/hooks/receive";

fn main() {
    let world = Arc::new(VastarRuntimeWorld::new_shared()
        .expect("Failed to initialize VIL SHM Runtime"));

    let sink_builder = HttpSinkBuilder::new("WebhookReceiver")
        .port(LISTEN_PORT)
        .path(LISTEN_PATH)
        .out_port("trigger_out")
        .in_port("data_in")
        .ctrl_in_port("ctrl_in");

    let source_builder = HttpSourceBuilder::new("Downstream")
        .url(DOWNSTREAM_URL)
        .format(HttpFormat::Json)
        .in_port("trigger_in")
        .out_port("data_out")
        .ctrl_out_port("ctrl_out")
        .post_json(serde_json::json!({"forward": true}));

    let (_ir, (sink_handle, source_handle)) = vil_workflow! {
        name: "WebhookForwarder",
        instances: [ sink_builder, source_builder ],
        routes: [
            sink_builder.trigger_out -> source_builder.trigger_in (LoanWrite),
            source_builder.data_out -> sink_builder.data_in (LoanWrite),
            source_builder.ctrl_out -> sink_builder.ctrl_in (Copy),
        ]
    };

    println!("VIL Webhook Forwarder");
    println!("  Listen:     http://localhost:{}{}", LISTEN_PORT, LISTEN_PATH);
    println!("  Downstream: {}", DOWNSTREAM_URL);
    println!();

    let sink = HttpSink::from_builder(sink_builder);
    let source = HttpSource::from_builder(source_builder);

    let t1 = sink.run_worker::<GenericToken>(world.clone(), sink_handle);
    let t2 = source.run_worker::<GenericToken>(world.clone(), source_handle);

    t1.join().expect("Sink worker panicked");
    t2.join().expect("Source worker panicked");
}
