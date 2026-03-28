# Messaging Integrations

VIL provides Kafka, NATS, and MQTT bridges for event-driven architectures.

## Kafka (vil_mq_kafka)

```rust
use vil_mq_kafka::prelude::*;

// Producer
let producer = KafkaProducer::new()
    .brokers("localhost:9092")
    .topic("orders")
    .build()?;

producer.send("order-123", &order_event).await?;

// Consumer
let consumer = KafkaConsumer::new()
    .brokers("localhost:9092")
    .group_id("order-processor")
    .topic("orders")
    .build()?;

consumer.subscribe(|msg: KafkaMessage| async move {
    let order: OrderEvent = msg.json()?;
    process_order(order).await
}).await?;
```

### Kafka Bridge (Pipeline Integration)

```rust
let kafka_source = KafkaBridge::source()
    .brokers("localhost:9092")
    .topic("incoming")
    .build();

let (_ir, handles) = vil_workflow! {
    name: "KafkaPipeline",
    token: ShmToken,
    instances: [ kafka_source, sink ],
    routes: [ kafka_source.data -> sink.in (LoanWrite) ]
};
```

## NATS (vil_mq_nats)

```rust
use vil_mq_nats::prelude::*;

let client = NatsClient::new()
    .url("nats://localhost:4222")
    .build()
    .await?;

// Publish
client.publish("events.order", &event).await?;

// Subscribe
client.subscribe("events.>", |msg: NatsMessage| async move {
    let event: Event = msg.json()?;
    handle_event(event).await
}).await?;

// Request-Reply
let response = client.request("api.users.get", &request).await?;
```

### JetStream (Durable)

```rust
let js = client.jetstream().await?;

// Create stream
js.create_stream(StreamConfig {
    name: "ORDERS".to_string(),
    subjects: vec!["orders.>".to_string()],
    ..Default::default()
}).await?;

// Durable consumer
let consumer = js.consumer("ORDERS", "processor").await?;
consumer.consume(|msg| async move {
    process(msg).await?;
    msg.ack().await
}).await?;
```

## MQTT (vil_mq_mqtt)

```rust
use vil_mq_mqtt::prelude::*;

let client = MqttClient::new()
    .url("mqtt://localhost:1883")
    .client_id("sensor-gateway")
    .build()
    .await?;

// Publish
client.publish("sensors/temperature", &reading, QoS::AtLeastOnce).await?;

// Subscribe
client.subscribe("sensors/#", |msg: MqttMessage| async move {
    let reading: SensorReading = msg.json()?;
    store_reading(reading).await
}).await?;
```

## VilApp Integration

Register as service state:

```rust
let service = ServiceProcess::new("events")
    .extension(kafka_producer)
    .extension(nats_client)
    .endpoint(Method::POST, "/publish", post(publish_event));

#[vil_handler(shm)]
async fn publish_event(ctx: ServiceCtx, slice: ShmSlice) -> VilResponse<()> {
    let event: Event = slice.json()?;
    let kafka = ctx.state::<KafkaProducer>();
    kafka.send(&event.topic, &event).await?;
    VilResponse::ok(())
}
```

## Additional Messaging Connectors

For messaging systems beyond Kafka/NATS/MQTT, see the native connector crates:

| System | Connector |
|--------|-----------|
| RabbitMQ (AMQP) | vil_conn_rabbitmq |
| AWS SQS/SNS | vil_conn_sqs |
| Apache Pulsar | vil_conn_pulsar |
| Google Pub/Sub | vil_conn_pubsub |

Full reference: [connectors/messaging.md](../connectors/messaging.md)

> Reference: docs/vil/006-VIL-Developer_Guide-CLI-Deployment.md
