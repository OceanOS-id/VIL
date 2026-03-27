# vil_mq_rabbitmq

VIL RabbitMQ Adapter — AMQP publish/consume with semantic log integration.

## Boundary Classification

| Path | Copy/Zero-Copy | Notes |
|------|---------------|-------|
| Publish (external) | Copy | AMQP wire protocol |
| Consume (external) | Copy | AMQP delivery deserialized |
| Internal pipeline  | Zero-copy via ExchangeHeap | After receive, place into SHM |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Message arrival notification |
| Data | Inbound → VIL | Message payload (via ExchangeHeap) |
| Control | Bidirectional | Ack/Nack (outbound), Error/Reconnect (inbound) |

## Thread Count

Each `consume()` call spawns 1 internal tokio task.
Add 1 per active consumer to `LogConfig.threads`.

## Quick Start

```rust
use vil_mq_rabbitmq::{RabbitConfig, RabbitClient};

let config = RabbitConfig::new(
    "amqp://guest:guest@localhost:5672/%2F",
    "my-exchange",
    "my-queue",
);
let client = RabbitClient::connect(config).await?;
client.publish("my-exchange", "routing.key", b"hello").await?;
let mut rx = client.consume("my-queue").await?;
if let Some(msg) = rx.recv().await {
    client.ack(msg.delivery_tag).await?;
}
```
