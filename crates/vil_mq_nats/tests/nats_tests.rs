// =============================================================================
// V10 NATS Adapter — Unit Tests
// =============================================================================

// ==================== Config Tests ====================

#[test]
fn test_nats_config() {
    use vil_mq_nats::NatsConfig;
    let config = NatsConfig::new("nats://localhost:4222")
        .with_token("secret-token")
        .tls(true)
        .name("my-service");

    assert_eq!(config.url, "nats://localhost:4222");
    assert!(config.credentials.is_some());
    assert!(config.tls);
    assert_eq!(config.client_name, "my-service");
    assert_eq!(config.max_reconnects, 60);
}

#[test]
fn test_nats_config_userpass() {
    use vil_mq_nats::NatsConfig;
    let config = NatsConfig::new("nats://localhost:4222")
        .with_userpass("admin", "password");

    let creds = config.credentials.unwrap();
    assert_eq!(creds.username, Some("admin".into()));
    assert_eq!(creds.password, Some("password".into()));
}

// ==================== Core Client Tests ====================

#[tokio::test]
async fn test_nats_connect() {
    use vil_mq_nats::{NatsClient, NatsConfig};
    let client = NatsClient::connect(NatsConfig::new("nats://localhost:4222")).await.unwrap();
    assert!(client.is_connected());
    assert_eq!(client.published_count(), 0);
}

#[tokio::test]
async fn test_nats_publish() {
    use vil_mq_nats::{NatsClient, NatsConfig};
    let client = NatsClient::connect(NatsConfig::new("nats://localhost:4222")).await.unwrap();

    client.publish("orders.created", b"order-1").await.unwrap();
    client.publish("orders.updated", b"order-2").await.unwrap();
    assert_eq!(client.published_count(), 2);
}

#[tokio::test]
async fn test_nats_pub_sub() {
    use vil_mq_nats::{NatsClient, NatsConfig};
    let client = NatsClient::connect(NatsConfig::new("nats://localhost:4222")).await.unwrap();

    // Subscribe first
    let mut sub = client.subscribe("test.subject").await.unwrap();
    assert_eq!(sub.subject(), "test.subject");

    // Publish
    client.publish("test.subject", b"hello nats").await.unwrap();

    // Receive
    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(100), sub.next()
    ).await.unwrap().unwrap();

    assert_eq!(msg.subject, "test.subject");
    assert_eq!(&msg.payload[..], b"hello nats");
}

#[tokio::test]
async fn test_nats_wildcard_subscribe() {
    use vil_mq_nats::{NatsClient, NatsConfig};
    let client = NatsClient::connect(NatsConfig::new("nats://localhost:4222")).await.unwrap();

    // Subscribe with > wildcard
    let mut sub = client.subscribe("orders.>").await.unwrap();

    // Publish to matching subjects
    client.publish("orders.created", b"new").await.unwrap();

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(100), sub.next()
    ).await.unwrap().unwrap();

    assert_eq!(msg.subject, "orders.created");
}

#[tokio::test]
async fn test_nats_request_reply() {
    use vil_mq_nats::{NatsClient, NatsConfig};
    let client = NatsClient::connect(NatsConfig::new("nats://localhost:4222")).await.unwrap();

    let reply = client.request("auth.verify", b"token-123").await.unwrap();
    assert!(!reply.payload.is_empty());
}

#[tokio::test]
async fn test_nats_disconnect() {
    use vil_mq_nats::{NatsClient, NatsConfig};
    let client = NatsClient::connect(NatsConfig::new("nats://localhost:4222")).await.unwrap();
    assert!(client.is_connected());

    client.disconnect().await;
    assert!(!client.is_connected());

    let result = client.publish("test", b"fail").await;
    assert!(result.is_err());
}

// ==================== JetStream Tests ====================

#[tokio::test]
async fn test_jetstream_create_stream() {
    use vil_mq_nats::jetstream::{JetStreamClient, StreamConfig};

    let js = JetStreamClient::new();
    assert_eq!(js.stream_count(), 0);

    js.create_stream(StreamConfig {
        name: "ORDERS".into(),
        subjects: vec!["orders.>".into()],
        retention: "limits".into(),
        max_msgs: 1_000_000,
        max_bytes: -1,
    }).await.unwrap();

    assert_eq!(js.stream_count(), 1);
    assert!(js.streams().contains(&"ORDERS".to_string()));
}

#[tokio::test]
async fn test_jetstream_publish_consume() {
    use vil_mq_nats::jetstream::{JetStreamClient, StreamConfig, ConsumerConfig};

    let js = JetStreamClient::new();

    js.create_stream(StreamConfig {
        name: "EVENTS".into(),
        subjects: vec!["events.>".into()],
        retention: "limits".into(),
        max_msgs: -1,
        max_bytes: -1,
    }).await.unwrap();

    let mut consumer = js.create_consumer("EVENTS", ConsumerConfig {
        durable_name: Some("event-processor".into()),
        filter_subject: None,
        ack_policy: "explicit".into(),
        deliver_policy: "all".into(),
    }).await.unwrap();

    // Publish
    let seq = js.publish("events.created", b"event-data").await.unwrap();
    assert!(seq > 0);

    // Consume
    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(100), consumer.next()
    ).await.unwrap().unwrap();

    assert_eq!(msg.subject, "events.created");
    assert_eq!(&msg.payload[..], b"event-data");
    assert!(!msg.is_acked());

    // Ack
    msg.ack().await.unwrap();
    assert!(msg.is_acked());
}

// ==================== KV Store Tests ====================

#[tokio::test]
async fn test_kv_put_get() {
    use vil_mq_nats::kv::KvStore;

    let kv = KvStore::new("config");
    assert!(kv.is_empty());

    kv.put("feature.dark_mode", b"true").await.unwrap();
    assert_eq!(kv.len(), 1);

    let entry = kv.get("feature.dark_mode").await.unwrap();
    assert_eq!(entry.key, "feature.dark_mode");
    assert_eq!(&entry.value[..], b"true");
    assert!(entry.revision > 0);
}

#[tokio::test]
async fn test_kv_delete() {
    use vil_mq_nats::kv::KvStore;
    let kv = KvStore::new("test");

    kv.put("key1", b"val1").await.unwrap();
    assert_eq!(kv.len(), 1);

    assert!(kv.delete("key1").await);
    assert_eq!(kv.len(), 0);
    assert!(kv.get("key1").await.is_none());
}

#[tokio::test]
async fn test_kv_keys() {
    use vil_mq_nats::kv::KvStore;
    let kv = KvStore::new("multi");

    kv.put("a", b"1").await.unwrap();
    kv.put("b", b"2").await.unwrap();
    kv.put("c", b"3").await.unwrap();

    let keys = kv.keys();
    assert_eq!(keys.len(), 3);
}

#[tokio::test]
async fn test_kv_watch() {
    use vil_mq_nats::kv::KvStore;

    let kv = KvStore::new("watched");
    let mut watcher = kv.watch();

    kv.put("key", b"value").await.unwrap();

    let entry = tokio::time::timeout(
        std::time::Duration::from_millis(100), watcher.recv()
    ).await.unwrap().unwrap();

    assert_eq!(entry.key, "key");
    assert_eq!(&entry.value[..], b"value");
}

// ==================== Bridge Tests ====================

#[tokio::test]
async fn test_nats_bridge() {
    use vil_mq_nats::NatsBridge;

    let bridge = NatsBridge::new("order-handler");
    assert_eq!(bridge.bridged_count(), 0);
    assert_eq!(bridge.target(), "order-handler");

    bridge.bridge("orders.created", b"order-data").await;
    bridge.bridge("orders.updated", b"update-data").await;
    assert_eq!(bridge.bridged_count(), 2);
}

// ==================== Metrics Tests ====================

#[test]
fn test_nats_metrics() {
    use vil_mq_nats::metrics::NatsMetrics;
    let m = NatsMetrics::new();
    let prom = m.to_prometheus();
    assert!(prom.contains("vil_nats_published"));
    assert!(prom.contains("vil_nats_js_published"));
    assert!(prom.contains("vil_nats_kv_puts"));
    assert!(prom.contains("vil_nats_bridged"));
}

// ==================== Health Tests ====================

#[tokio::test]
async fn test_nats_health() {
    use vil_mq_nats::{NatsClient, NatsConfig};
    use vil_mq_nats::health::check_health;

    let client = NatsClient::connect(NatsConfig::new("nats://localhost:4222")).await.unwrap();
    let (healthy, msg) = check_health(&client).await;
    assert!(healthy);
    assert_eq!(msg, "connected");

    client.disconnect().await;
    let (healthy, _) = check_health(&client).await;
    assert!(!healthy);
}
