// =============================================================================
// example-502-villog-file-rolling — VIL Log file drain with rolling rotation
// =============================================================================
//
// Demonstrates writing structured logs to a rolling JSON-Lines file.
//
// Configuration:
//   - Daily rotation strategy
//   - Max 7 retained files
//   - Output dir: ./logs/
//
// After running, inspect ./logs/app.log for JSON Lines output.
// =============================================================================

use std::path::PathBuf;

use vil_log::drain::{FileDrain, RotationStrategy};
use vil_log::runtime::init_logging;
use vil_log::{
    app_log, access_log, db_log,
    AccessPayload, DbPayload,
    LogConfig, LogLevel,
};

#[tokio::main]
async fn main() {
    // Write logs to ./logs/ relative to cwd
    let log_dir = PathBuf::from("./logs");

    let drain = FileDrain::new(
        &log_dir,
        "app",
        RotationStrategy::Daily,
        7, // keep last 7 files
    )
    .expect("failed to create log dir");

    let config = LogConfig {
        ring_slots:        8192,
        level:             LogLevel::Debug,
        batch_size:        256,
        flush_interval_ms: 50,
        threads: None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };

    let _task = init_logging(config, drain);

    println!("Writing 100 log events to {}/app.log ...", log_dir.display());

    // Emit 100 application events
    for i in 0u32..50 {
        app_log!(Info, "order.processed", {
            order_id: i as u64,
            amount: (i * 1000) as u64,
            currency: "IDR"
        });
        app_log!(Debug, "order.detail", {
            order_id: i as u64,
            items: i + 1u32
        });
    }

    // Emit access logs
    for i in 0u32..25 {
        let status: u16 = if i % 10 == 0 { 500 } else { 200 };
        access_log!(Info, AccessPayload {
            method:         0, // GET
            status_code:    status,
            protocol:       0,
            duration_us:    100 + i * 10,
            request_bytes:  64,
            response_bytes: 512 + i * 8,
            route_hash:     register_str("/api/orders"),
            path_hash:      register_str("/api/orders"),
            authenticated:  1,
            ..AccessPayload::default()
        });
    }

    // Emit database logs
    for i in 0u32..25 {
        db_log!(Info, DbPayload {
            db_hash:      register_str("postgres"),
            table_hash:   register_str("orders"),
            query_hash:   register_str("SELECT * FROM orders WHERE id = $1"),
            duration_us:  500 + i * 20,
            rows_affected: 1,
            op_type:      0, // SELECT
            prepared:     1,
            tx_state:     0, // none
            error_code:   0,
            ..DbPayload::default()
        });
    }

    // Flush and wait
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    println!("Done. Log file at: {}/app.log", log_dir.display());
    println!("Preview (first 3 lines):");

    if let Ok(content) = std::fs::read_to_string(log_dir.join("app.log")) {
        for line in content.lines().take(3) {
            println!("  {}", line);
        }
    }
}
