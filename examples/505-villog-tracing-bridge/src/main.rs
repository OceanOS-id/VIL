// =============================================================================
// example-505-villog-tracing-bridge — VilTracingLayer ecosystem bridge
// =============================================================================
//
// Shows how `VilTracingLayer` bridges the tracing ecosystem into the VIL ring.
//
// Any library that emits `tracing::info!`, `tracing::warn!`, etc. will
// automatically have its events captured in the VIL ring — without
// changing a single line of library code.
//
// Setup:
//   tracing_subscriber::registry()
//     .with(VilTracingLayer::new())
//     .init();
//
// After setup, tracing events flow through VilTracingLayer → VIL ring → drain.
// =============================================================================

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{app_log, LogConfig, LogLevel, VilTracingLayer};

// Simulates a third-party library using tracing
mod third_party_lib {
    pub fn process_request(request_id: u64) {
        tracing::info!(request_id, "processing request");
        tracing::debug!(request_id, step = "validate", "input validated");
        tracing::debug!(request_id, step = "enrich",   "data enriched");
        tracing::info!(request_id, "request complete");
    }

    pub fn authenticate(user_id: u64, success: bool) {
        if success {
            tracing::info!(user_id, "authentication successful");
        } else {
            tracing::warn!(user_id, "authentication failed");
        }
    }

    pub fn database_error(query: &str, error: &str) {
        tracing::error!(query, error, "database query failed");
    }
}

#[tokio::main]
async fn main() {
    // 1. Set up VIL drain first (ring must exist before tracing subscriber)
    let config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Debug,
        batch_size:        64,
        flush_interval_ms: 50,
        threads: None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let drain = StdoutDrain::new(StdoutFormat::Compact);
    let _task = init_logging(config, drain);

    // 2. Install VilTracingLayer as the global tracing subscriber
    tracing_subscriber::registry()
        .with(VilTracingLayer::new())
        .init();

    println!("=== VilTracingLayer bridge demo ===");
    println!("tracing events from libraries -> VIL ring -> StdoutDrain\n");

    // 3. VIL native logs still work alongside bridged tracing events
    app_log!(Info, "bridge.demo.start", { version: "0.1.0" });

    // 4. Call "library" code — it uses tracing, but output goes via VIL
    third_party_lib::process_request(42);
    third_party_lib::authenticate(1001, true);
    third_party_lib::authenticate(1002, false);
    third_party_lib::database_error(
        "SELECT * FROM sessions WHERE token = $1",
        "connection pool exhausted",
    );

    // 5. Direct tracing calls also flow through
    tracing::info!(component = "main", "bridge demo complete");

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    println!("\n=== All tracing events captured in VIL ring ===");
}
