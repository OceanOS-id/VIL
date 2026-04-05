// =============================================================================
// example-805-trigger-email — HR Employee Notification System
// =============================================================================
//
// Domain:   Human Resources
// Business: Email trigger fires when a new email arrives at the HR inbox,
//           processes the content (e.g. leave requests, onboarding docs).
//
// Demonstrates:
//   - EmailConfig with IMAP credentials
//   - EmailTrigger via vil_trigger_email::process::create_trigger()
//   - TriggerSource::start() with an EventCallback + mpsc relay
//   - Receiving TriggerEvent descriptors from the mpsc Receiver
//   - mq_log! auto-emitted by vil_trigger_email on every email arrival
//   - StdoutDrain::resolved() output
//
// The example collects 3 email events then exits.
// Requires an IMAP server (or set env vars to point at one).
//
// Environment variables:
//   IMAP_HOST     — IMAP server hostname        (default: imap.example.com)
//   IMAP_PORT     — IMAP server port             (default: 993)
//   IMAP_USER     — IMAP account username        (default: hr-inbox@acme.com)
//   IMAP_PASS     — IMAP account password        (default: changeme)
//   IMAP_FOLDER   — Mailbox folder to watch      (default: INBOX)
// =============================================================================

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerSource};
use vil_trigger_email::{EmailConfig, EmailTrigger};

/// Number of email events to collect before stopping.
const EVENT_COUNT: u32 = 3;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let log_config = LogConfig {
        ring_slots: 4096,
        level: LogLevel::Info,
        batch_size: 64,
        flush_interval_ms: 50,
        threads: None,
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let _task = init_logging(log_config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-805-trigger-email");
    println!("  HR Employee Notification System — Email trigger (collects {} events then exits)", EVENT_COUNT);
    println!();

    // ── Build EmailConfig from env ──
    let email_cfg = EmailConfig::new(
        env_or("IMAP_HOST", "imap.example.com"),
        env_or("IMAP_PORT", "993").parse::<u16>().unwrap_or(993),
        env_or("IMAP_USER", "hr-inbox@acme.com"),
        env_or("IMAP_PASS", "changeme"),
        env_or("IMAP_FOLDER", "INBOX"),
    );

    println!("  IMAP host  : {}:{}", email_cfg.imap_host, email_cfg.port);
    println!("  Folder     : {}", email_cfg.folder);
    println!();

    // ── Create the trigger ──
    let trigger = Arc::new(EmailTrigger::new(email_cfg));

    // ── Wire up mpsc channel for downstream consumption ──
    let (tx, mut rx) = tokio::sync::mpsc::channel::<TriggerEvent>(64);

    // Fire counter shared with callback
    let fires = Arc::new(AtomicU32::new(0));
    let fires_cb = fires.clone();

    // Callback: prints each TriggerEvent and relays to mpsc
    let on_event: EventCallback = Arc::new(move |event: TriggerEvent| {
        let n = fires_cb.fetch_add(1, Ordering::Relaxed) + 1;
        println!(
            "  FIRE #{n}  seq={}  ts_ns={}  kind_hash={:#010x}  [HR email received]",
            event.sequence, event.timestamp_ns, event.kind_hash,
        );
        let _ = tx.try_send(event);
    });

    // ── Start the trigger in a background task ──
    let trigger_bg = trigger.clone();
    tokio::spawn(async move {
        if let Err(e) = trigger_bg.start(on_event).await {
            println!("  Trigger stopped with fault: {:?}", e);
        }
    });

    println!(
        "  Waiting for {} email events (IMAP IDLE)...",
        EVENT_COUNT
    );
    println!();

    // ── Drain the mpsc receiver — process HR notifications ──
    let mut received = 0u32;
    while received < EVENT_COUNT {
        match tokio::time::timeout(std::time::Duration::from_secs(120), rx.recv()).await {
            Ok(Some(event)) => {
                received += 1;
                println!(
                    "  RECV  seq={}  payload_bytes={}  op={}  [processing HR notification #{}/{}]",
                    event.sequence, event.payload_bytes, event.op, received, EVENT_COUNT
                );
                if received >= EVENT_COUNT {
                    break;
                }
            }
            Ok(None) => {
                println!("  Channel closed");
                break;
            }
            Err(_) => {
                println!("  Timeout waiting for email event (120s)");
                break;
            }
        }
    }

    // ── Stop the trigger ──
    if let Err(e) = trigger.stop().await {
        println!("  Stop fault: {:?}", e);
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!(
        "  Done. {} HR email events collected. mq_log! entries emitted above.",
        received
    );
    println!();
}
