// =============================================================================
// Kafka Producer — real rdkafka FutureProducer
// =============================================================================

use crate::config::KafkaConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub struct KafkaProducer {
    producer: FutureProducer,
    config: KafkaConfig,
    messages_sent: AtomicU64,
    errors: AtomicU64,
}

impl KafkaProducer {
    pub async fn new(config: KafkaConfig) -> Result<Self, String> {
        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &config.brokers);
        client_config.set("message.timeout.ms", &config.timeout_ms.to_string());
        client_config.set("acks", &config.acks);

        // Apply SASL/security if configured
        if let Some(ref protocol) = config.security_protocol {
            client_config.set("security.protocol", protocol);
        }
        if let Some(ref mechanism) = config.sasl_mechanism {
            client_config.set("sasl.mechanism", mechanism);
        }
        if let Some(ref username) = config.sasl_username {
            client_config.set("sasl.username", username);
        }
        if let Some(ref password) = config.sasl_password {
            client_config.set("sasl.password", password);
        }

        let producer: FutureProducer = client_config.create()
            .map_err(|e| format!("Kafka producer creation failed: {}", e))?;

        tracing::info!(brokers = %config.brokers, "kafka producer created (real rdkafka)");
        Ok(Self {
            producer,
            config,
            messages_sent: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        })
    }

    /// Publish a message to a topic.
    pub async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), String> {
        let __mq_start = std::time::Instant::now();
        let record: FutureRecord<'_, str, [u8]> = FutureRecord::to(topic)
            .payload(payload);
        let result = self.producer.send(record, Duration::from_millis(self.config.timeout_ms))
            .await
            .map_err(|(e, _)| {
                self.errors.fetch_add(1, Ordering::Relaxed);
                format!("Kafka publish failed: {}", e)
            });
        if result.is_ok() {
            self.messages_sent.fetch_add(1, Ordering::Relaxed);
        }
        let __elapsed = __mq_start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(Info, MqPayload {
                broker_hash: register_str("kafka"),
                topic_hash: register_str(topic),
                message_bytes: payload.len() as u32,
                e2e_latency_us: __elapsed.as_micros() as u32,
                op_type: 0,
                ..Default::default()
            });
        }
        tracing::debug!(topic = %topic, size = payload.len(), "kafka publish");
        result.map(|_| ())
    }

    /// Publish with key (for partitioning).
    pub async fn publish_keyed(&self, topic: &str, key: &str, payload: &[u8]) -> Result<(), String> {
        let __mq_start = std::time::Instant::now();
        let record = FutureRecord::to(topic)
            .payload(payload)
            .key(key);
        let result = self.producer.send(record, Duration::from_millis(self.config.timeout_ms))
            .await
            .map_err(|(e, _)| {
                self.errors.fetch_add(1, Ordering::Relaxed);
                format!("Kafka publish_keyed failed: {}", e)
            });
        if result.is_ok() {
            self.messages_sent.fetch_add(1, Ordering::Relaxed);
        }
        let __elapsed = __mq_start.elapsed();
        {
            use vil_log::{mq_log, types::MqPayload};
            mq_log!(Info, MqPayload {
                broker_hash: register_str("kafka"),
                topic_hash: register_str(topic),
                message_bytes: payload.len() as u32,
                e2e_latency_us: __elapsed.as_micros() as u32,
                op_type: 0,
                ..Default::default()
            });
        }
        tracing::debug!(topic = %topic, key = %key, size = payload.len(), "kafka publish keyed");
        result.map(|_| ())
    }

    pub fn messages_sent(&self) -> u64 { self.messages_sent.load(Ordering::Relaxed) }
    pub fn errors(&self) -> u64 { self.errors.load(Ordering::Relaxed) }
    pub fn config(&self) -> &KafkaConfig { &self.config }

    /// Access the underlying rdkafka FutureProducer.
    pub fn inner(&self) -> &FutureProducer { &self.producer }
}
