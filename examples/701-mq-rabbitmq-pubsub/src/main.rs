// =============================================================================
// example-701-mq-rabbitmq-pubsub — RabbitMQ publish + consume with mq_log!
// =============================================================================
//
// Demonstrates:
//   - RabbitClient::connect() with a local AMQP URI
//   - publish() to a queue (empty exchange = direct to queue)
//   - consume() from a queue (returns mpsc Receiver<RabbitMessage>)
//   - ack() per received message
//   - mq_log! auto-emitted by vil_mq_rabbitmq on every operation
//   - StdoutDrain::resolved() output
//
// Requires: RabbitMQ running locally.
// Quick start:
//   docker run -p 5672:5672 -p 15672:15672 rabbitmq:3-management
//
// Without Docker, this example prints config and exits gracefully.
// =============================================================================

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_mq_rabbitmq::{RabbitClient, RabbitConfig};

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let config = LogConfig {
        ring_slots: 4096,
        level: LogLevel::Info,
        batch_size: 64,
        flush_interval_ms: 50,
        threads: Some(2),
        dict_path: None,
        fallback_path: None,
        drain_failure_threshold: 3,
    };
    let _task = init_logging(config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-701-mq-rabbitmq-pubsub");
    println!("  RabbitMQ publish + consume with mq_log! auto-emit");
    println!();

    let rabbit_cfg = RabbitConfig::new(
        "amqp://guest:guest@localhost:5672/%2F",
        "vil.results",
        "vil.tasks",
    );

    println!("  Connecting to RabbitMQ: {}", rabbit_cfg.uri);
    println!(
        "  Exchange: {}  Queue: {}",
        rabbit_cfg.exchange, rabbit_cfg.queue
    );
    println!();
    println!("  NOTE: Requires RabbitMQ running locally.");
    println!("  Start with:");
    println!("    docker run -p 5672:5672 -p 15672:15672 rabbitmq:3-management");
    println!();

    let client = match RabbitClient::connect(rabbit_cfg.clone()).await {
        Ok(c) => c,
        Err(e) => {
            println!("  [SKIP] Cannot connect to RabbitMQ: {:?}", e);
            println!("  (All mq_log! calls would appear above in resolved format)");
            return;
        }
    };

    // ── PUBLISH 3 messages (direct to queue via empty exchange) ──
    for i in 1u32..=3 {
        let body = format!(r#"{{"task_id":{},"payload":"process-batch-{i}"}}"#, i * 100);
        match client.publish("", &rabbit_cfg.queue, body.as_bytes()).await {
            Ok(_) => println!("  PUBLISH [{}] {}", i, body),
            Err(e) => println!("  PUBLISH error: {:?}", e),
        }
    }

    // ── CONSUME up to 3 messages ──
    println!();
    let mut rx = match client.consume(&rabbit_cfg.queue).await {
        Ok(r) => r,
        Err(e) => {
            println!("  CONSUME error: {:?}", e);
            return;
        }
    };

    println!("  Consuming (up to 3 messages)...");
    for _ in 0..3 {
        match tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await {
            Ok(Some(msg)) => {
                let body = String::from_utf8_lossy(&msg.payload);
                println!(
                    "  RECEIVE  delivery_tag={}  body={}",
                    msg.delivery_tag, body
                );
                // Acknowledge
                if let Err(e) = client.ack(msg.delivery_tag).await {
                    println!("  ACK     error: {:?}", e);
                }
            }
            Ok(None) => {
                println!("  Channel closed");
                break;
            }
            Err(_) => {
                println!("  Timeout — no more messages");
                break;
            }
        }
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. mq_log! entries emitted above in resolved format.");
    println!();
}
