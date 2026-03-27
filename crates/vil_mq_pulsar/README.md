# vil_mq_pulsar

VIL Apache Pulsar Adapter — producer/consumer with semantic log integration.

## Boundary Classification

| Path | Copy/Zero-Copy | Notes |
|------|---------------|-------|
| Send/Receive (external) | Copy | Pulsar wire protocol |
| Internal pipeline | Zero-copy via ExchangeHeap | After receive, place into SHM |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Message arrival notification |
| Data | Inbound → VIL | Message payload (via ExchangeHeap) |
| Control | Bidirectional | Ack (outbound), Error/Reconnect (inbound) |

## Thread Count

Each active `PulsarConsumer` drives 1 tokio task.
Add 1 per active consumer to `LogConfig.threads`.

## Quick Start

```rust
use vil_mq_pulsar::{PulsarConfig, PulsarClient, PulsarProducer, PulsarConsumer};

let config = PulsarConfig::new("pulsar://localhost:6650", "public", "default");
let client = PulsarClient::connect(config).await?;

let mut producer = PulsarProducer::new(&client, "my-topic").await?;
producer.send(b"hello pulsar").await?;

let mut consumer = PulsarConsumer::new(&client, "my-topic", "my-subscription").await?;
let msg = consumer.receive().await?;
consumer.ack(&msg.message_id).await?;
```
