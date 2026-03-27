// =============================================================================
// V9 MQTT Adapter — Unit Tests
// =============================================================================

#[test]
fn test_mqtt_config() {
    use vil_mq_mqtt::{MqttConfig, QoS};
    let config = MqttConfig::new("mqtt://localhost:1883")
        .client_id("test-client")
        .qos(QoS::ExactlyOnce)
        .tls(true);

    assert_eq!(config.broker_url, "mqtt://localhost:1883");
    assert_eq!(config.client_id, Some("test-client".into()));
    assert_eq!(config.qos, QoS::ExactlyOnce);
    assert!(config.tls);
    assert_eq!(config.port, 1883);
    assert_eq!(config.keepalive_secs, 60);
}

#[test]
fn test_qos_default() {
    use vil_mq_mqtt::QoS;
    assert_eq!(QoS::default(), QoS::AtLeastOnce);
}

#[test]
fn test_qos_values() {
    use vil_mq_mqtt::QoS;
    assert_eq!(QoS::AtMostOnce as u8, 0);
    assert_eq!(QoS::AtLeastOnce as u8, 1);
    assert_eq!(QoS::ExactlyOnce as u8, 2);
}

#[tokio::test]
async fn test_mqtt_client_lifecycle() {
    use vil_mq_mqtt::{MqttClient, MqttConfig, QoS};

    let client = MqttClient::new(MqttConfig::new("mqtt://localhost:1883")).await.unwrap();
    assert!(client.is_connected());
    assert_eq!(client.published_count(), 0);

    client.publish("sensors/temp", b"25.5", QoS::AtLeastOnce).await.unwrap();
    assert_eq!(client.published_count(), 1);

    client.subscribe("sensors/+/temperature").await.unwrap();

    client.disconnect().await;
    assert!(!client.is_connected());

    // Publish after disconnect should fail
    let result = client.publish("test", b"fail", QoS::AtMostOnce).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mqtt_bridge() {
    use vil_mq_mqtt::MqttBridge;
    let bridge = MqttBridge::new("sensor-processor");
    assert_eq!(bridge.bridged_count(), 0);

    bridge.bridge("sensors/temp", b"25.5").await;
    bridge.bridge("sensors/humidity", b"60").await;
    assert_eq!(bridge.bridged_count(), 2);
}
