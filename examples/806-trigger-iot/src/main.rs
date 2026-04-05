// =============================================================================
// example-806-trigger-iot — Manufacturing Assembly Line Monitoring
// =============================================================================
//
// Domain:   Manufacturing
// Business: IoT sensor trigger fires when MQTT messages arrive from assembly
//           line sensors, detecting threshold breaches (e.g. temperature,
//           vibration, pressure exceeding safe limits).
//
// Demonstrates:
//   - IotConfig with MQTT broker settings
//   - IotTrigger via vil_trigger_iot (rumqttc-based)
//   - TriggerSource::start() with an EventCallback + mpsc relay
//   - Receiving TriggerEvent descriptors from the mpsc Receiver
//   - mq_log! auto-emitted by vil_trigger_iot on every MQTT publish
//   - StdoutDrain::resolved() output
//
// The example collects 5 sensor events then exits.
// Requires an MQTT broker (or set env vars to point at one).
//
// Environment variables:
//   MQTT_HOST      — MQTT broker hostname        (default: localhost)
//   MQTT_PORT      — MQTT broker port             (default: 1883)
//   MQTT_TOPIC     — Topic filter to subscribe    (default: factory/line-a/+/threshold)
//   MQTT_CLIENT_ID — MQTT client identifier       (default: vil-iot-assembly-01)
// =============================================================================

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_trigger_core::{EventCallback, TriggerEvent, TriggerSource};
use vil_trigger_iot::{IotConfig, IotTrigger};

/// Number of sensor events to collect before stopping.
const EVENT_COUNT: u32 = 5;

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
    println!("  example-806-trigger-iot");
    println!("  Manufacturing Assembly Line Monitoring — IoT sensor trigger (collects {} events then exits)", EVENT_COUNT);
    println!();

    // ── Build IotConfig from env ──
    let iot_cfg = IotConfig::new(
        env_or("MQTT_HOST", "localhost"),
        env_or("MQTT_PORT", "1883").parse::<u16>().unwrap_or(1883),
        env_or("MQTT_TOPIC", "factory/line-a/+/threshold"),
        env_or("MQTT_CLIENT_ID", "vil-iot-assembly-01"),
    );

    println!("  MQTT broker : {}:{}", iot_cfg.mqtt_host, iot_cfg.port);
    println!("  Topic       : {}", iot_cfg.topic);
    println!("  Client ID   : {}", iot_cfg.client_id);
    println!();

    // ── Create the trigger ──
    let trigger = Arc::new(IotTrigger::new(iot_cfg));

    // ── Wire up mpsc channel for downstream consumption ──
    let (tx, mut rx) = tokio::sync::mpsc::channel::<TriggerEvent>(128);

    // Fire counter shared with callback
    let fires = Arc::new(AtomicU32::new(0));
    let fires_cb = fires.clone();

    // Callback: prints each TriggerEvent and relays to mpsc
    let on_event: EventCallback = Arc::new(move |event: TriggerEvent| {
        let n = fires_cb.fetch_add(1, Ordering::Relaxed) + 1;
        println!(
            "  FIRE #{n}  seq={}  ts_ns={}  payload={}B  kind_hash={:#010x}  [sensor threshold breach]",
            event.sequence, event.timestamp_ns, event.payload_bytes, event.kind_hash,
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
        "  Waiting for {} sensor events (MQTT subscription)...",
        EVENT_COUNT
    );
    println!();

    // ── Drain the mpsc receiver — process assembly line alerts ──
    let mut received = 0u32;
    while received < EVENT_COUNT {
        match tokio::time::timeout(std::time::Duration::from_secs(60), rx.recv()).await {
            Ok(Some(event)) => {
                received += 1;

                // Classify severity based on payload size as a proxy
                let severity = if event.payload_bytes > 512 {
                    "CRITICAL"
                } else if event.payload_bytes > 128 {
                    "WARNING"
                } else {
                    "NORMAL"
                };

                println!(
                    "  RECV  seq={}  payload_bytes={}  op={}  severity={}  [sensor alert #{}/{}]",
                    event.sequence, event.payload_bytes, event.op, severity, received, EVENT_COUNT
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
                println!("  Timeout waiting for sensor event (60s)");
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
        "  Done. {} assembly line sensor events collected. mq_log! entries emitted above.",
        received
    );
    println!();
}
