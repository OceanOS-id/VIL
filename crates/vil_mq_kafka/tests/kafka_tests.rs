// =============================================================================
// V9 Kafka Adapter — Unit Tests
// =============================================================================

#[test]
fn test_kafka_config() {
    use vil_mq_kafka::KafkaConfig;
    let config = KafkaConfig::new("localhost:9092")
        .group("my-group")
        .topic("orders");

    assert_eq!(config.brokers, "localhost:9092");
    assert_eq!(config.group_id, Some("my-group".into()));
    assert_eq!(config.topic, Some("orders".into()));
    assert_eq!(config.acks, "all");
    assert_eq!(config.timeout_ms, 5000);
}

#[tokio::test]
async fn test_kafka_producer() {
    use vil_mq_kafka::{KafkaProducer, KafkaConfig};
    let producer = KafkaProducer::new(KafkaConfig::new("localhost:9092")).await.unwrap();

    assert_eq!(producer.messages_sent(), 0);

    producer.publish("test-topic", b"hello kafka").await.unwrap();
    assert_eq!(producer.messages_sent(), 1);

    producer.publish_keyed("test-topic", "key-1", b"keyed msg").await.unwrap();
    assert_eq!(producer.messages_sent(), 2);
}

#[tokio::test]
async fn test_kafka_consumer() {
    use vil_mq_kafka::{KafkaConsumer, KafkaConfig};
    use vil_mq_kafka::consumer::KafkaMessage;
    use bytes::Bytes;

    let mut consumer = KafkaConsumer::new(
        KafkaConfig::new("localhost:9092").group("test").topic("orders")
    ).await.unwrap();

    assert!(!consumer.is_running());
    consumer.start();
    assert!(consumer.is_running());

    // Inject a test message
    consumer.inject_message(KafkaMessage {
        topic: "orders".into(), partition: 0, offset: 1,
        key: Some("k1".into()), payload: Bytes::from("test payload"),
    }).await;

    assert_eq!(consumer.messages_received(), 1);

    consumer.stop();
    assert!(!consumer.is_running());
}

#[tokio::test]
async fn test_kafka_bridge() {
    use vil_mq_kafka::KafkaBridge;
    use vil_mq_kafka::consumer::KafkaMessage;
    use bytes::Bytes;

    let bridge = KafkaBridge::new("order-service");
    assert_eq!(bridge.bridged_count(), 0);
    assert_eq!(bridge.target_service(), "order-service");

    let msg = KafkaMessage {
        topic: "orders".into(), partition: 0, offset: 1,
        key: None, payload: Bytes::from("data"),
    };
    bridge.bridge(&msg).await;
    assert_eq!(bridge.bridged_count(), 1);
}

#[test]
fn test_kafka_metrics() {
    use vil_mq_kafka::metrics::KafkaMetrics;
    let m = KafkaMetrics::new();
    let prom = m.to_prometheus();
    assert!(prom.contains("vil_kafka_produced_total"));
    assert!(prom.contains("vil_kafka_consumed_total"));
}
