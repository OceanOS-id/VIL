// =============================================================================
// example-803-trigger-webhook-receiver — HTTP Webhook receiver with HMAC
// =============================================================================
//
// Demonstrates:
//   - create_trigger() building a WebhookTrigger
//   - TriggerSource::start() binding an HTTP listener on port 8090
//   - HMAC-SHA256 verification (secret="" disables check for demo simplicity)
//   - mq_log! auto-emitted by vil_trigger_webhook on every valid POST
//   - StdoutDrain::resolved() output
//
// After starting, the example sends 3 test webhook POSTs to itself using
// reqwest, then exits after 5 seconds.
//
// No external services required.
//
// Production HMAC usage:
//   Add hmac="0.12" + sha2="0.10" + hex="0.4" deps, then sign payloads and
//   set SECRET to a non-empty value. The server checks X-Hub-Signature-256.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerSource};
use vil_trigger_webhook::{WebhookConfig, WebhookTrigger};

const LISTEN_ADDR: &str = "0.0.0.0:8090";
const WEBHOOK_PATH: &str = "/webhook";
/// Empty secret disables HMAC verification — convenient for local demos.
/// Set to a non-empty string to enforce HMAC-SHA256 (X-Hub-Signature-256).
const SECRET: &str = "";

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
    println!("  example-803-trigger-webhook-receiver");
    println!("  Webhook receiver with mq_log! auto-emit");
    println!();
    println!("  Listen:  http://{}{}", LISTEN_ADDR, WEBHOOK_PATH);
    println!("  Secret:  {} (empty = no HMAC check)", if SECRET.is_empty() { "(none)" } else { SECRET });
    println!();

    let cfg = WebhookConfig::new(LISTEN_ADDR, SECRET, WEBHOOK_PATH);
    let trigger: Arc<dyn TriggerSource> = Arc::new(WebhookTrigger::new(cfg));

    // Event counter
    let event_count = Arc::new(AtomicU32::new(0));
    let event_count_cb = event_count.clone();

    let on_event: EventCallback = Arc::new(move |event: TriggerEvent| {
        let n = event_count_cb.fetch_add(1, Ordering::Relaxed) + 1;
        println!("  WEBHOOK #{n}  seq={}  payload_bytes={}  ts_ns={}",
            event.sequence,
            event.payload_bytes,
            event.timestamp_ns,
        );
    });

    // Start the webhook server in background
    let trigger_bg = trigger.clone();
    let on_event_bg = on_event.clone();
    tokio::spawn(async move {
        if let Err(e) = trigger_bg.start(on_event_bg).await {
            println!("  Webhook server stopped: {:?}", e);
        }
    });

    // Allow server to bind
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    println!("  Server listening. Sending 3 test POSTs...");
    println!();

    // ── Send 3 test webhook POSTs to ourselves ──
    let http = reqwest::Client::new();
    let target_url = format!("http://127.0.0.1:8090{}", WEBHOOK_PATH);

    for i in 1u32..=3 {
        let body = serde_json::json!({
            "event": "order.created",
            "id":    i * 100,
            "amount": i * 50_000u32,
        }).to_string();

        match http
            .post(&target_url)
            .header("Content-Type", "application/json")
            .body(body.clone())
            .send()
            .await
        {
            Ok(resp) => println!("  POST [{}]  status={}  body={:.60}", i, resp.status(), body),
            Err(e)   => println!("  POST [{}]  error: {}", i, e),
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // Wait for all events to be processed
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Stop the trigger
    if let Err(e) = trigger.stop().await {
        println!("  Stop fault: {:?}", e);
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. {} webhook events received. mq_log! entries emitted above.",
        event_count.load(Ordering::Relaxed));
    println!();
}
