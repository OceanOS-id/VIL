// =============================================================================
// Kafka Consumer — real rdkafka StreamConsumer
// =============================================================================

use crate::config::KafkaConfig;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use bytes::Bytes;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use rdkafka::Message as RdMessage;
use futures::StreamExt;

/// Consumed Kafka message.
#[derive(Debug, Clone)]
pub struct KafkaMessage {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub payload: Bytes,
}

pub struct KafkaConsumer {
    consumer: Arc<StreamConsumer>,
    config: KafkaConfig,
    messages_received: AtomicU64,
    running: AtomicBool,
    tx: mpsc::Sender<KafkaMessage>,
    rx: Option<mpsc::Receiver<KafkaMessage>>,
}

impl KafkaConsumer {
    pub async fn new(config: KafkaConfig) -> Result<Self, String> {
        let group_id = config.group_id.as_deref().unwrap_or("vil-default-group");

        let mut client_config = ClientConfig::new();
        client_config.set("bootstrap.servers", &config.brokers);
        client_config.set("group.id", group_id);
        client_config.set("enable.auto.commit", "false");
        client_config.set("auto.offset.reset", "earliest");

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

        let consumer: StreamConsumer = client_config.create()
            .map_err(|e| format!("Kafka consumer creation failed: {}", e))?;

        // Subscribe to topic if configured
        if let Some(ref topic) = config.topic {
            consumer.subscribe(&[topic])
                .map_err(|e| format!("Kafka topic subscribe failed: {}", e))?;
        }

        let (tx, rx) = mpsc::channel(1024);

        Ok(Self {
            consumer: Arc::new(consumer),
            config,
            messages_received: AtomicU64::new(0),
            running: AtomicBool::new(false),
            tx, rx: Some(rx),
        })
    }

    /// Take the receiver (for bridging to Tri-Lane).
    pub fn take_receiver(&mut self) -> Option<mpsc::Receiver<KafkaMessage>> {
        self.rx.take()
    }

    /// Inject a test message (for testing without a real broker).
    pub async fn inject_message(&self, msg: KafkaMessage) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        let _ = self.tx.send(msg).await;
    }

    /// Start consuming in a background task.
    /// Messages are forwarded to the mpsc channel (use take_receiver to consume).
    pub fn start(&self) {
        if self.running.swap(true, Ordering::Relaxed) {
            return; // already running
        }
        let consumer = self.consumer.clone();
        let tx = self.tx.clone();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let _received_counter = AtomicU64::new(0);

        tokio::spawn(async move {
            let mut stream = consumer.stream();
            while running_clone.load(Ordering::Relaxed) {
                match stream.next().await {
                    Some(Ok(borrowed_msg)) => {
                        let topic_str = RdMessage::topic(&borrowed_msg).to_string();
                        let payload_bytes = RdMessage::payload(&borrowed_msg).map(|p| Bytes::copy_from_slice(p)).unwrap_or_default();
                        let payload_len = payload_bytes.len();
                        let partition = RdMessage::partition(&borrowed_msg);
                        let offset = RdMessage::offset(&borrowed_msg);
                        {
                            use vil_log::{mq_log, types::MqPayload};
                            mq_log!(Info, MqPayload {
                                broker_hash: register_str("kafka"),
                                topic_hash: register_str(&topic_str),
                                message_bytes: payload_len as u32,
                                op_type: 1,
                                partition: partition.clamp(0, 255) as u8,
                                offset: offset.max(0) as u64,
                                ..Default::default()
                            });
                        }
                        let kafka_msg = KafkaMessage {
                            topic: topic_str,
                            partition,
                            offset,
                            key: RdMessage::key(&borrowed_msg).map(|k| String::from_utf8_lossy(k).to_string()),
                            payload: payload_bytes,
                        };
                        if tx.send(kafka_msg).await.is_err() {
                            break; // receiver dropped
                        }
                    }
                    Some(Err(_e)) => {
                    }
                    None => break,
                }
            }
        });
    }

    /// Stop consuming.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool { self.running.load(Ordering::Relaxed) }
    pub fn messages_received(&self) -> u64 { self.messages_received.load(Ordering::Relaxed) }
}
