// =============================================================================
// example-603-db-clickhouse-batch — ClickHouse batch INSERT with BatchInserter
// =============================================================================
//
// Demonstrates:
//   - ChClient::new() with a local ClickHouse config
//   - BatchInserter<T> for buffered batch INSERT
//   - db_log! auto-emitted by vil_db_clickhouse on flush
//   - StdoutDrain::resolved() output
//
// Requires: ClickHouse running locally.
// Quick start:
//   docker run -p 8123:8123 -p 9000:9000 clickhouse/clickhouse-server
//
// Without Docker, this example prints config and exits gracefully.
// =============================================================================

use std::time::Duration;

use clickhouse::Row;
use serde::Serialize;
use vil_db_clickhouse::{BatchInserter, ChClient, ClickHouseConfig};
use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};

/// Demo row for the `events` table.
#[derive(Debug, Clone, Row, Serialize)]
struct EventRow {
    /// Epoch timestamp in seconds.
    #[serde(rename = "ts")]
    ts:       u64,
    /// Event type (0=click, 1=view, 2=purchase).
    #[serde(rename = "event_type")]
    event_type: u8,
    /// User identifier.
    #[serde(rename = "user_id")]
    user_id:  u64,
    /// Associated value (e.g. price in cents).
    #[serde(rename = "value")]
    value:    u32,
}

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Info,
        batch_size:        64,
        flush_interval_ms: 50,
        threads:           None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let _task = init_logging(config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-603-db-clickhouse-batch");
    println!("  ClickHouse batch INSERT with BatchInserter + db_log! auto-emit");
    println!();

    let ch_cfg = ClickHouseConfig {
        url:      "http://localhost:8123".into(),
        database: "vil_demo".into(),
        username: Some("default".into()),
        password: None,
    };

    println!("  Connecting to ClickHouse: {}", ch_cfg.url);
    println!("  Database: {}", ch_cfg.database);
    println!();
    println!("  NOTE: Requires ClickHouse running locally.");
    println!("  Start with:");
    println!("    docker run -p 8123:8123 -p 9000:9000 clickhouse/clickhouse-server");
    println!();

    let client = ChClient::new(ch_cfg.clone());

    // Create table if needed
    let ddl = "CREATE TABLE IF NOT EXISTS vil_demo.events \
               (ts UInt64, event_type UInt8, user_id UInt64, value UInt32) \
               ENGINE = MergeTree() ORDER BY ts";

    match client.execute(ddl).await {
        Ok(_)  => println!("  DDL     events table ready"),
        Err(e) => {
            println!("  [SKIP] Cannot connect to ClickHouse: {:?}", e);
            println!("  (All db_log! calls would appear above in resolved format)");
            return;
        }
    }

    // ── BatchInserter: flush every 5 rows or 1 second ──
    let mut inserter: BatchInserter<EventRow> = BatchInserter::new(
        ChClient::new(ch_cfg),
        "vil_demo.events",
        5,
        Duration::from_secs(1),
    );

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    println!("  Pushing 12 rows (auto-flush at 5 rows)...");
    for i in 0u64..12 {
        let row = EventRow {
            ts:         now + i,
            event_type: (i % 3) as u8,
            user_id:    1000 + i,
            value:      (i as u32) * 100,
        };
        match inserter.push(row).await {
            Ok(_)  => {}
            Err(e) => { println!("  push error: {:?}", e); return; }
        }
    }

    // Flush remaining rows
    match inserter.flush().await {
        Ok(_)  => println!("  Final flush complete"),
        Err(e) => println!("  flush error: {:?}", e),
    }

    // Allow drain to flush
    tokio::time::sleep(Duration::from_millis(200)).await;
    println!();
    println!("  Done. db_log! entries (with rows_affected) emitted above.");
    println!();
}
