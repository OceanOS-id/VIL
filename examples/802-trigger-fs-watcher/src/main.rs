// =============================================================================
// example-802-trigger-fs-watcher — Filesystem watcher trigger
// =============================================================================
//
// Demonstrates:
//   - create_fs_trigger() watching a temporary directory
//   - TriggerSource::start() with an EventCallback
//   - Receiving TriggerEvent descriptors as files are created/modified
//   - mq_log! auto-emitted by vil_trigger_fs on every file event
//   - StdoutDrain::resolved() output
//
// Creates a temp directory, writes several files, then exits after 5 events
// (or a timeout).  No external services required.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerSource};
use vil_trigger_fs::{FsConfig, FsEventMask, create_fs_trigger};

/// Watch path — must be &'static str; we use a known temp location.
/// The directory is created at startup if it doesn't exist.
const WATCH_DIR: &str = "/tmp/vil-802-watch";

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
    println!("  example-802-trigger-fs-watcher");
    println!("  Filesystem watcher trigger with mq_log! auto-emit");
    println!();
    println!("  Watch directory: {}", WATCH_DIR);
    println!();

    // Ensure watch directory exists
    if let Err(e) = std::fs::create_dir_all(WATCH_DIR) {
        println!("  [ERROR] Cannot create watch dir: {}", e);
        return;
    }
    println!("  Directory ready: {}", WATCH_DIR);

    let fs_cfg = FsConfig {
        trigger_id:       2,
        watch_path:       WATCH_DIR,
        pattern:          Some("*.log"),
        debounce_ms:      100,
        recursive:        false,
        events:           FsEventMask::all(),
        channel_capacity: 256,
    };

    let (trigger, mut rx) = create_fs_trigger(fs_cfg);
    let trigger = Arc::new(trigger);

    // Event counter
    let event_count = Arc::new(AtomicU32::new(0));
    let event_count_cb = event_count.clone();

    let on_event: EventCallback = Arc::new(move |event: TriggerEvent| {
        let n = event_count_cb.fetch_add(1, Ordering::Relaxed) + 1;
        println!("  FS EVENT #{n}  seq={}  source_hash={:#010x}  payload_bytes={}",
            event.sequence,
            event.source_hash,
            event.payload_bytes,
        );
    });

    // Start watcher in background
    let trigger_bg = trigger.clone();
    tokio::spawn(async move {
        if let Err(e) = trigger_bg.start(on_event).await {
            println!("  Watcher stopped with fault: {:?}", e);
        }
    });

    // Allow watcher to initialize
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    println!("  Writing 5 .log files to trigger events...");
    for i in 1u32..=5 {
        let path = format!("{}/event-{:03}.log", WATCH_DIR, i);
        let content = format!("vil-802 test event {}\n", i);
        if let Err(e) = std::fs::write(&path, &content) {
            println!("  write error: {}", e);
        } else {
            println!("  Wrote: {}", path);
        }
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }

    // Collect up to 5 events from the channel
    println!();
    println!("  Collecting events from channel (timeout 5s)...");
    let mut received = 0u32;
    while received < 5 {
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            rx.recv(),
        ).await {
            Ok(Some(event)) => {
                received += 1;
                println!("  RECV  seq={}  op={}", event.sequence, event.op);
            }
            Ok(None) => { println!("  Channel closed"); break; }
            Err(_)   => { println!("  Timeout"); break; }
        }
    }

    // Stop the watcher
    if let Err(e) = trigger.stop().await {
        println!("  Stop fault: {:?}", e);
    }

    // Cleanup: remove created files
    for i in 1u32..=5 {
        let path = format!("{}/event-{:03}.log", WATCH_DIR, i);
        let _ = std::fs::remove_file(&path);
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. {} events received. mq_log! entries emitted above.", received);
    println!();
}
