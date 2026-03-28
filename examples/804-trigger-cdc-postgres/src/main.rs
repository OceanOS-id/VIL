// =============================================================================
// example-804-trigger-cdc-postgres — PostgreSQL CDC trigger
// =============================================================================
//
// Demonstrates:
//   - create_trigger() building a CdcTrigger for PostgreSQL logical replication
//   - TriggerSource::start() begins streaming row-change events
//   - mq_log! auto-emitted by vil_trigger_cdc on every row change
//   - StdoutDrain::resolved() output
//
// Requires: PostgreSQL with logical replication enabled + a replication slot.
//
// Docker setup:
//   docker run -p 5432:5432 \
//     -e POSTGRES_PASSWORD=secret \
//     -e POSTGRES_DB=vildb \
//     postgres:16 \
//     postgres -c wal_level=logical \
//              -c max_replication_slots=4 \
//              -c max_wal_senders=4
//
//   # Then in psql:
//   psql -h localhost -U postgres -d vildb
//   CREATE TABLE orders (id SERIAL PRIMARY KEY, amount INT, status TEXT);
//   CREATE PUBLICATION vil_pub FOR TABLE orders;
//   SELECT pg_create_logical_replication_slot('vil_cdc_slot', 'pgoutput');
//
// Without Docker, this example documents the setup and exits gracefully.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_trigger_cdc::{CdcConfig, process::create_trigger};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerSource};

const CONN_STRING: &str =
    "host=localhost port=5432 dbname=vildb user=postgres password=secret";
const SLOT_NAME: &str = "vil_cdc_slot";
const PUBLICATION: &str = "vil_pub";

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let log_config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Info,
        batch_size:        64,
        flush_interval_ms: 50,
        threads:           None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let _task = init_logging(log_config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-804-trigger-cdc-postgres");
    println!("  PostgreSQL CDC trigger with mq_log! auto-emit");
    println!();
    println!("  Connection: {}", CONN_STRING);
    println!("  Slot:       {}", SLOT_NAME);
    println!("  Publication:{}", PUBLICATION);
    println!();

    println!("  Docker setup:");
    println!("    docker run -p 5432:5432 \\");
    println!("      -e POSTGRES_PASSWORD=secret \\");
    println!("      -e POSTGRES_DB=vildb \\");
    println!("      postgres:16 \\");
    println!("      postgres -c wal_level=logical \\");
    println!("               -c max_replication_slots=4 \\");
    println!("               -c max_wal_senders=4");
    println!();
    println!("    # In psql:");
    println!("    CREATE TABLE orders (id SERIAL PRIMARY KEY, amount INT, status TEXT);");
    println!("    CREATE PUBLICATION vil_pub FOR TABLE orders;");
    println!("    SELECT pg_create_logical_replication_slot('vil_cdc_slot', 'pgoutput');");
    println!();

    let cdc_cfg = CdcConfig::new(CONN_STRING, SLOT_NAME, PUBLICATION);
    let trigger: Arc<dyn TriggerSource> = create_trigger(cdc_cfg);

    // Event counter
    let event_count = Arc::new(AtomicU32::new(0));
    let event_count_cb = event_count.clone();

    let on_event: EventCallback = Arc::new(move |event: TriggerEvent| {
        let n = event_count_cb.fetch_add(1, Ordering::Relaxed) + 1;
        println!("  CDC ROW #{n}  seq={}  source_hash={:#010x}  payload_bytes={}",
            event.sequence,
            event.source_hash,
            event.payload_bytes,
        );
    });

    println!("  Starting CDC trigger (will fail if Postgres is not running)...");

    let trigger_bg = trigger.clone();
    let handle = tokio::spawn(async move {
        if let Err(e) = trigger_bg.start(on_event).await {
            println!("  CDC trigger stopped: {:?}", e);
        }
    });

    // Wait a short time — if Postgres is available, events start flowing
    // immediately as rows are inserted. If not, the trigger will fault.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    println!();
    println!("  (To generate CDC events, INSERT into the orders table while running)");
    println!("  Example:  INSERT INTO orders (amount, status) VALUES (50000, 'pending');");
    println!();

    // Stop the trigger
    if let Err(e) = trigger.stop().await {
        println!("  Stop fault: {:?}", e);
    }
    handle.abort();

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let total = event_count.load(Ordering::Relaxed);
    if total > 0 {
        println!("  Done. {} CDC row change events captured.", total);
    } else {
        println!("  Done. No events captured (Postgres/CDC not configured or not reachable).");
        println!("  In production, each INSERT/UPDATE/DELETE emits mq_log! with:");
        println!("    op_type=1(consume), source_hash=<slot_hash>, payload_bytes=<row_size>");
    }
    println!();
}
