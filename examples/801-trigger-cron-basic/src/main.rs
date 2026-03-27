// =============================================================================
// example-801-trigger-cron-basic — Cron trigger firing every 5 seconds
// =============================================================================
//
// Demonstrates:
//   - create_cron_trigger() with a 5-second schedule
//   - TriggerSource::start() with an EventCallback
//   - Receiving TriggerEvent descriptors from the mpsc Receiver
//   - mq_log! auto-emitted by vil_trigger_cron on every fire
//   - StdoutDrain::resolved() output
//
// The example fires 3 times (every 5 seconds) then exits.
// No external services required.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerSource};
use vil_trigger_cron::{CronConfig, create_cron_trigger};

/// Number of cron fires to collect before stopping.
const FIRE_COUNT: u32 = 3;

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let log_config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Info,
        batch_size:        64,
        flush_interval_ms: 50,
        threads:           None,
    };
    let _task = init_logging(log_config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-801-trigger-cron-basic");
    println!("  Cron trigger firing every 5 seconds (collects 3 fires then exits)");
    println!();

    // 6-field cron: "*/5 * * * * *" fires every 5 seconds
    let cron_cfg = CronConfig::new(1, "*/5 * * * * *");

    let (trigger, mut rx) = match create_cron_trigger(cron_cfg) {
        Ok(pair) => pair,
        Err(e)   => {
            println!("  [ERROR] Invalid cron schedule: {:?}", e);
            return;
        }
    };

    // Fire counter shared with callback
    let fires = Arc::new(AtomicU32::new(0));
    let fires_cb = fires.clone();

    // Callback prints each TriggerEvent as it fires
    let on_event: EventCallback = Arc::new(move |event: TriggerEvent| {
        let n = fires_cb.fetch_add(1, Ordering::Relaxed) + 1;
        println!("  FIRE #{n}  seq={}  ts_ns={}  kind_hash={:#010x}",
            event.sequence,
            event.timestamp_ns,
            event.kind_hash,
        );
    });

    // Start the trigger in a background task
    let trigger = Arc::new(trigger);
    let trigger_arc = trigger.clone();

    tokio::spawn(async move {
        if let Err(e) = trigger_arc.start(on_event).await {
            println!("  Trigger stopped with fault: {:?}", e);
        }
    });

    println!("  Waiting for {} cron fires (every 5 seconds)...", FIRE_COUNT);
    println!();

    // Drain the mpsc receiver channel — TriggerEvents also flow here
    let mut received = 0u32;
    while received < FIRE_COUNT {
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            rx.recv(),
        ).await {
            Ok(Some(event)) => {
                received += 1;
                println!("  RECV  seq={}  payload_bytes={}  op={}",
                    event.sequence, event.payload_bytes, event.op);
                if received >= FIRE_COUNT {
                    break;
                }
            }
            Ok(None) => {
                println!("  Channel closed");
                break;
            }
            Err(_) => {
                println!("  Timeout waiting for cron fire");
                break;
            }
        }
    }

    // Stop the trigger
    if let Err(e) = trigger.stop().await {
        println!("  Stop fault: {:?}", e);
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. {} fires collected. mq_log! entries emitted above.", received);
    println!();
}
