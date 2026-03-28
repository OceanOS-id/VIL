// =============================================================================
// example-503-villog-multi-drain — VIL Log fan-out to stdout + file
// =============================================================================
//
// Demonstrates MultiDrain routing each log batch to multiple drains in order:
//   1. StdoutDrain (compact format) — for live monitoring
//   2. FileDrain (daily rotation)   — for persistent storage
//
// Every log event is written to both destinations simultaneously.
// =============================================================================

use std::path::PathBuf;

use vil_log::drain::{FileDrain, MultiDrain, RotationStrategy, StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{
    app_log, access_log, mq_log,
    AccessPayload, MqPayload,
    LogConfig, LogLevel,
};

#[tokio::main]
async fn main() {
    let log_dir = PathBuf::from("./logs");

    // Build file drain — size-based rotation at 10MB, keep 5 files
    let file_drain = FileDrain::new(
        &log_dir,
        "multi",
        RotationStrategy::Size { max_bytes: 10 * 1024 * 1024 },
        5,
    )
    .expect("failed to create log dir");

    // Fan-out: compact stdout + file
    let multi = MultiDrain::new()
        .add(StdoutDrain::new(StdoutFormat::Compact))
        .add(file_drain);

    let config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Info,
        batch_size:        128,
        flush_interval_ms: 50,
        threads: None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };

    let _task = init_logging(config, multi);

    println!("=== Multi-drain: stdout (compact) + file ===\n");

    // Application events
    app_log!(Info,  "service.start",   { version: "0.1.0", env: "staging" });
    app_log!(Info,  "user.registered", { user_id: 1001u64, plan: "free" });
    app_log!(Warn,  "rate.limited",    { user_id: 1001u64, endpoint: "/api/upload" });
    app_log!(Error, "webhook.failed",  { target: "https://example.com/hook", status: 503u32 });

    // HTTP access events
    for i in 0u32..5 {
        access_log!(Info, AccessPayload {
            method:         1, // POST
            status_code:    200,
            protocol:       1, // HTTP/2
            duration_us:    800 + i * 50,
            request_bytes:  1024,
            response_bytes: 256,
            route_hash:     register_str("/api/events"),
            path_hash:      register_str("/api/events"),
            authenticated:  1,
            ..AccessPayload::default()
        });
    }

    // Message queue events
    mq_log!(Info, MqPayload {
        broker_hash:    register_str("kafka"),
        topic_hash:     register_str("order.created"),
        group_hash:     register_str("order-processor"),
        offset:         1_042_883,
        message_bytes:  512,
        e2e_latency_us: 3_200,
        op_type:        1, // consume
        partition:      0,
        retries:        0,
        ..MqPayload::default()
    });

    mq_log!(Warn, MqPayload {
        broker_hash:    register_str("kafka"),
        topic_hash:     register_str("payment.failed"),
        group_hash:     register_str("payment-processor"),
        offset:         9_875,
        message_bytes:  256,
        e2e_latency_us: 12_000,
        op_type:        4, // dlq
        partition:      2,
        retries:        3,
        ..MqPayload::default()
    });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    println!("\n=== Fan-out complete. Also check {}/multi.log ===", log_dir.display());
}
