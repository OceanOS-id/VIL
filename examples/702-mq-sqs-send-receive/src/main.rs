// =============================================================================
// example-702-mq-sqs-send-receive — AWS SQS send/receive (LocalStack)
// =============================================================================
//
// Demonstrates:
//   - SqsClient::from_config() with LocalStack endpoint
//   - send_message, receive_messages, delete_message
//   - mq_log! auto-emitted by vil_mq_sqs on every operation
//   - StdoutDrain::resolved() output
//
// Requires: LocalStack (or real AWS credentials) running locally.
// Quick start (LocalStack):
//   docker run -p 4566:4566 localstack/localstack
//   # Then create the queue:
//   aws --endpoint-url=http://localhost:4566 sqs create-queue --queue-name vil-tasks
//
// Without LocalStack, this example prints config and exits gracefully.
// =============================================================================

use vil_log::drain::{StdoutDrain, StdoutFormat};
use vil_log::runtime::init_logging;
use vil_log::{LogConfig, LogLevel};
use vil_mq_sqs::{SqsClient, SqsConfig};

const LOCALSTACK_ENDPOINT: &str = "http://localhost:4566";
const QUEUE_URL: &str = "http://localhost:4566/000000000000/vil-tasks";

#[tokio::main]
async fn main() {
    // ── Init vil_log with resolved drain ──
    let config = LogConfig {
        ring_slots:        4096,
        level:             LogLevel::Info,
        batch_size:        64,
        flush_interval_ms: 50,
        threads:           None,
    };
    let _task = init_logging(config, StdoutDrain::new(StdoutFormat::Resolved));

    println!();
    println!("  example-702-mq-sqs-send-receive");
    println!("  AWS SQS send/receive with LocalStack + mq_log! auto-emit");
    println!();

    let sqs_cfg = SqsConfig::new("us-east-1", QUEUE_URL)
        .with_endpoint(LOCALSTACK_ENDPOINT)
        .with_max_messages(5);

    println!("  Endpoint:  {}", LOCALSTACK_ENDPOINT);
    println!("  Queue URL: {}", QUEUE_URL);
    println!();
    println!("  NOTE: Requires LocalStack running locally.");
    println!("  Start with:");
    println!("    docker run -p 4566:4566 localstack/localstack");
    println!("    aws --endpoint-url=http://localhost:4566 sqs create-queue \\");
    println!("        --queue-name vil-tasks");
    println!();

    let client = match SqsClient::from_config(sqs_cfg).await {
        Ok(c)  => c,
        Err(e) => {
            println!("  [SKIP] Cannot build SQS client: {:?}", e);
            println!("  (All mq_log! calls would appear above in resolved format)");
            return;
        }
    };

    // ── SEND 3 messages ──
    for i in 1u32..=3 {
        let body = format!(r#"{{"job_id":{},"type":"etl","batch":{i}}}"#, i * 10);
        match client.send_message(body.as_bytes()).await {
            Ok(_)  => println!("  SEND [{}] {}", i, body),
            Err(e) => {
                println!("  SEND error: {:?}", e);
                println!("  [SKIP] SQS queue not reachable.");
                return;
            }
        }
    }

    // ── RECEIVE messages ──
    println!();
    println!("  Receiving messages...");
    match client.receive_messages().await {
        Ok(messages) => {
            println!("  Received {} message(s)", messages.len());
            for msg in &messages {
                let body = String::from_utf8_lossy(&msg.body);
                println!("  RECV  receive_count={}  body={}", msg.receive_count, body);

                // Delete (acknowledge) the message
                match client.delete_message(&msg.receipt_handle).await {
                    Ok(_)  => println!("  DEL   receipt_handle={:.20}...", msg.receipt_handle),
                    Err(e) => println!("  DEL   error: {:?}", e),
                }
            }
        }
        Err(e) => println!("  RECEIVE error: {:?}", e),
    }

    // Allow drain to flush
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!();
    println!("  Done. mq_log! entries emitted above in resolved format.");
    println!();
}
