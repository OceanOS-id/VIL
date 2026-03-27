# vil_mq_sqs

VIL AWS SQS/SNS Adapter — send, receive, delete, and SNS publish with semantic log integration.

## Boundary Classification

| Path | Copy/Zero-Copy | Notes |
|------|---------------|-------|
| Send/Receive (external) | Copy | AWS wire protocol |
| Internal pipeline | Zero-copy via ExchangeHeap | After receive, place into SHM |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Message arrival notification |
| Data | Inbound → VIL | Message payload (via ExchangeHeap) |
| Control | Bidirectional | Delete/ack (outbound), Error (inbound) |

## Quick Start

```rust
use vil_mq_sqs::{SqsConfig, SqsClient, SnsClient};

let config = SqsConfig::new("us-east-1", "https://sqs.us-east-1.amazonaws.com/123/my-queue");
let client = SqsClient::from_config(config.clone()).await?;
client.send_message(b"hello world").await?;

let messages = client.receive_messages().await?;
for msg in &messages {
    client.delete_message(&msg.receipt_handle).await?;
}

let sns = SnsClient::from_config(&config).await?;
sns.publish("arn:aws:sns:us-east-1:123:my-topic", b"event").await?;
```
