# vil_mq_pubsub

VIL Google Cloud Pub/Sub Adapter — publish and subscribe with semantic log integration.

## Boundary Classification

| Path | Copy/Zero-Copy | Notes |
|------|---------------|-------|
| Publish/Subscribe (external) | Copy | gRPC wire protocol |
| Internal pipeline | Zero-copy via ExchangeHeap | After receive, place into SHM |

## Tri-Lane Mapping

| Lane | Direction | Content |
|------|-----------|---------|
| Trigger | Inbound → VIL | Message arrival notification |
| Data | Inbound → VIL | Message payload (via ExchangeHeap) |
| Control | Bidirectional | Ack (outbound), Error (inbound) |

## Thread Count

Each active `subscribe()` polling loop uses 1 tokio task.
Add 1 per active subscriber to `LogConfig.threads`.

## Quick Start

```rust
use vil_mq_pubsub::{PubSubConfig, PubSubClient};

let config = PubSubConfig::new("my-project", "my-topic", "my-subscription");
let client = PubSubClient::new(config).await?;

client.publish(b"hello pubsub").await?;

let messages = client.subscribe().await?;
let ack_ids: Vec<String> = messages.iter().map(|m| m.ack_id.clone()).collect();
client.ack(&ack_ids).await?;
```

## Emulator (Local Testing)

```rust
let config = PubSubConfig::new("my-project", "my-topic", "my-sub")
    .with_emulator("localhost:8085");
```
