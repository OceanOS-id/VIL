# Messaging Connectors

VIL native messaging connectors (distinct from `integrations/messaging.md` which covers Kafka, NATS, MQTT). These connectors include `#[connector_fault/event/state]` instrumentation.

## Quick Reference

| Connector | Crate | Protocol |
|-----------|-------|----------|
| RabbitMQ | vil_conn_rabbitmq | AMQP 0-9-1 |
| SQS/SNS | vil_conn_sqs | AWS SQS + SNS |
| Pulsar | vil_conn_pulsar | Apache Pulsar |
| Google Pub/Sub | vil_conn_pubsub | GCP Pub/Sub |

Also see [integrations/messaging.md](../integrations/messaging.md) for Kafka, NATS, MQTT.

## RabbitMQ (vil_conn_rabbitmq)

```rust
use vil_conn_rabbitmq::{RabbitMqConnector, RabbitMqConfig};

let rabbit = RabbitMqConnector::new(RabbitMqConfig {
    url: "amqp://guest:guest@localhost:5672".into(),
    ..Default::default()
}).await?;

// Declare exchange + queue
rabbit.declare_exchange("orders", ExchangeKind::Direct).await?;
rabbit.declare_queue("orders.new").await?;
rabbit.bind_queue("orders.new", "orders", "new").await?;

// Publish
rabbit.publish("orders", "new", &order_event).await?;

// Consume
rabbit.consume("orders.new", |delivery: Delivery| async move {
    let order: OrderEvent = delivery.json()?;
    process_order(order).await?;
    delivery.ack().await
}).await?;
```

### Patterns

```rust
// Topic exchange (wildcard routing)
rabbit.declare_exchange("events", ExchangeKind::Topic).await?;
rabbit.bind_queue("events.payments", "events", "payment.#").await?;
rabbit.publish("events", "payment.created", &event).await?;

// Fanout (broadcast)
rabbit.declare_exchange("broadcast", ExchangeKind::Fanout).await?;
rabbit.publish("broadcast", "", &notification).await?;
```

## SQS/SNS (vil_conn_sqs)

```rust
use vil_conn_sqs::{SqsConnector, SnsConnector, SqsConfig, SnsConfig};

let sqs = SqsConnector::new(SqsConfig {
    queue_url: "https://sqs.us-east-1.amazonaws.com/123456789/my-queue".into(),
    region: "us-east-1".into(),
    ..Default::default()
}).await?;

// Send message
sqs.send(&order_event).await?;

// Send with delay
sqs.send_delayed(&order_event, Duration::from_secs(30)).await?;

// Poll and process (long-poll)
sqs.consume(|msg: SqsMessage| async move {
    let order: OrderEvent = msg.json()?;
    process_order(order).await?;
    msg.delete().await  // removes from queue
}).await?;
```

### SNS Fan-out

```rust
let sns = SnsConnector::new(SnsConfig {
    topic_arn: "arn:aws:sns:us-east-1:123:my-topic".into(),
    region: "us-east-1".into(),
    ..Default::default()
}).await?;

// Publish to SNS topic (fans out to all SQS subscriptions)
sns.publish(&event, None).await?;

// Publish with message attributes
sns.publish_with_attrs(&event, attrs! {
    "event_type" => ("String", "order.created")
}).await?;
```

## Apache Pulsar (vil_conn_pulsar)

```rust
use vil_conn_pulsar::{PulsarConnector, PulsarConfig};

let pulsar = PulsarConnector::new(PulsarConfig {
    url: "pulsar://localhost:6650".into(),
    tenant: "public".into(),
    namespace: "default".into(),
    ..Default::default()
}).await?;

// Producer
let producer = pulsar.producer("orders/new").await?;
producer.send(&order_event).await?;

// Consumer (exclusive)
let consumer = pulsar.consumer("orders/new")
    .subscription("order-processor")
    .subscription_type(SubscriptionType::Shared)
    .build().await?;

consumer.consume(|msg: PulsarMessage| async move {
    let order: OrderEvent = msg.json()?;
    process_order(order).await?;
    msg.ack().await
}).await?;
```

### Multi-topic subscription

```rust
let consumer = pulsar.consumer_multi(vec!["orders/new", "orders/retry"])
    .subscription("processor")
    .build().await?;
```

## Google Pub/Sub (vil_conn_pubsub)

```rust
use vil_conn_pubsub::{PubSubConnector, PubSubConfig};

let pubsub = PubSubConnector::new(PubSubConfig {
    project_id: "my-gcp-project".into(),
    credentials_json: std::fs::read_to_string("service_account.json")?,
    ..Default::default()
}).await?;

// Publish
pubsub.publish("orders-topic", &order_event).await?;

// Subscribe (pull)
pubsub.subscribe("orders-subscription", |msg: PubSubMessage| async move {
    let order: OrderEvent = msg.json()?;
    process_order(order).await?;
    msg.ack().await
}).await?;
```

## VilApp Integration

```rust
let service = ServiceProcess::new("events")
    .extension(rabbit.clone())
    .extension(sqs.clone())
    .endpoint(Method::POST, "/order", post(place_order));

#[vil_handler(shm)]
async fn place_order(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<()> {
    let order: OrderEvent = slice.json()?;
    let rabbit = ctx.state::<RabbitMqConnector>();
    rabbit.publish("orders", "new", &order).await?;
    VilResponse::ok(())
}
```

## Pipeline Source Integration

```rust
// Use RabbitMQ as pipeline source
let rabbit_source = RabbitMqBridge::source()
    .queue("orders.new")
    .connector(rabbit.clone())
    .build();

let (_ir, handles) = vil_workflow! {
    name: "OrderPipeline",
    token: ShmToken,
    instances: [ rabbit_source, processor ],
    routes: [ rabbit_source.data -> processor.in (LoanWrite) ]
};
```
